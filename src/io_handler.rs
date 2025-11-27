use anyhow::{Result, anyhow};
use bstr::ByteSlice;
use ecow::EcoString;
use memchr::memchr;
use tokio::process::Command as TokioCommand;

pub struct IoHandler;

impl IoHandler {
    pub async fn read_file(path: &str) -> Result<EcoString> {
        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| anyhow!("Failed to read file {}: {}", path, e))?;
        Ok(EcoString::from(content))
    }

    pub async fn read_from_command(cmd: &str) -> Result<EcoString> {
        let output = TokioCommand::new("sh")
            .arg("-c")
            .arg(cmd)
            .output()
            .await
            .map_err(|e| anyhow!("Failed to execute command: {}", e))?;

        if !output.status.success() {
            return Err(anyhow!("Command failed: {}", cmd));
        }

        Ok(EcoString::from(
            String::from_utf8_lossy(&output.stdout).to_string(),
        ))
    }

    pub async fn get_command_help(cmd: &str) -> Result<EcoString> {
        Self::read_from_command(&format!("{} --help 2>/dev/null || {}", cmd, cmd)).await
    }

    pub async fn get_manpage(cmd: &str) -> Result<EcoString> {
        Self::read_from_command(&format!("man {} 2>/dev/null | col -bx", cmd)).await
    }

    pub fn normalize_text(text: &str) -> EcoString {
        let bytes = text.as_bytes();

        // SIMD fast path: check if any tabs or double spaces exist
        let has_tabs = memchr(b'\t', bytes).is_some();

        // Quick check for double spaces - look for at least one space then another
        let has_double_spaces = {
            let mut found = false;
            let mut iter = bytes.iter().peekable();
            while let Some(&b) = iter.next() {
                if b == b' '
                    && let Some(&&next) = iter.peek()
                        && next == b' ' {
                            found = true;
                            break;
                        }
            }
            found
        };

        if !has_tabs && !has_double_spaces {
            return EcoString::from(text);
        }

        // Use bstr for SIMD-accelerated line iteration
        let mut result = String::with_capacity(text.len());
        let mut first = true;

        for line in bytes.lines() {
            if !first {
                result.push('\n');
            }
            first = false;

            // Safe conversion - original text is valid UTF-8
            let line_str = unsafe { std::str::from_utf8_unchecked(line) };

            // Apply transformations only if needed
            if has_tabs && has_double_spaces {
                let replaced = line_str.replace('\t', "        ").replace("  ", " ");
                result.push_str(&replaced);
            } else if has_tabs {
                result.push_str(&line_str.replace('\t', "        "));
            } else {
                result.push_str(&line_str.replace("  ", " "));
            }
        }

        EcoString::from(result)
    }

    pub async fn is_man_available(cmd: &str) -> bool {
        TokioCommand::new("man")
            .arg(cmd)
            .output()
            .await
            .map(|output| output.status.success())
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_text() {
        let input = "hello\t\tworld";
        let output = IoHandler::normalize_text(input);
        assert!(!output.contains('\t'));
    }

    #[tokio::test]
    async fn test_read_file() {
        use std::io::Write;

        let mut file = tempfile::NamedTempFile::new().expect("create temp file");
        write!(file, "hello world").expect("write temp file");
        let path = file.path().to_str().unwrap();

        let content = IoHandler::read_file(path).await.expect("read temp file");
        assert_eq!(content.as_str(), "hello world");

        let missing = IoHandler::read_file("/this/does/not/exist").await;
        assert!(missing.is_err());
    }

    #[tokio::test]
    async fn test_read_from_command() {
        let out = IoHandler::read_from_command("echo hello")
            .await
            .expect("run echo");
        assert!(out.contains("hello"));

        let res = IoHandler::read_from_command("exit 1").await;
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn test_get_command_help() {
        let help = IoHandler::get_command_help("echo").await.expect("get help");
        assert!(!help.is_empty());
    }

    #[tokio::test]
    async fn test_is_man_available() {
        let _man_available = IoHandler::is_man_available("echo").await;
        // Just test it runs without panic
    }

    #[tokio::test]
    async fn test_get_manpage() {
        if IoHandler::is_man_available("echo").await {
            let man = IoHandler::get_manpage("echo").await.expect("get manpage");
            assert!(!man.is_empty());
        }
    }
}
