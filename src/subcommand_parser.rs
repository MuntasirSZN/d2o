use crate::types::Subcommand;
use bstr::ByteSlice;
use ecow::{EcoString, EcoVec};
use std::collections::BTreeSet;

pub struct SubcommandParser;

impl SubcommandParser {
    pub fn parse(content: &str) -> EcoVec<Subcommand> {
        // Use bstr for SIMD-accelerated line iteration
        let bytes = content.as_bytes();
        let lines: Vec<&str> = bytes
            .lines()
            .filter_map(|line| std::str::from_utf8(line).ok())
            .collect();
        let mut subcommands = BTreeSet::new();

        for window in lines.windows(2) {
            if let Some(subcommand) = Self::parse_line_pair(window[0], window[1]) {
                subcommands.insert(subcommand);
            }
        }

        for line in &lines {
            if let Some(subcommand) = Self::parse_single_line(line) {
                subcommands.insert(subcommand);
            }
        }

        subcommands.into_iter().collect()
    }

    fn parse_line_pair(first: &str, second: &str) -> Option<Subcommand> {
        let trimmed_first = first.trim();
        let trimmed_bytes = trimmed_first.as_bytes();

        // Fast path: skip empty or option lines using byte check
        if trimmed_bytes.is_empty() || trimmed_bytes[0] == b'-' {
            return None;
        }

        let first_word = trimmed_first.split_whitespace().next()?;

        if !Self::is_valid_subcommand_name(first_word) {
            return None;
        }

        let desc = second.trim();
        let desc_bytes = desc.as_bytes();

        // Fast path: skip empty or option descriptions
        if desc_bytes.is_empty() || desc_bytes[0] == b'-' {
            return None;
        }

        // Use memchr to find newline if present
        let desc_line = match memchr::memchr(b'\n', desc_bytes) {
            Some(pos) => &desc[..pos],
            None => desc,
        };

        Some(Subcommand {
            cmd: EcoString::from(first_word),
            desc: EcoString::from(desc_line),
        })
    }

    fn parse_single_line(line: &str) -> Option<Subcommand> {
        let trimmed = line.trim();
        let trimmed_bytes = trimmed.as_bytes();

        // Fast path: skip empty or option lines using byte check
        if trimmed_bytes.is_empty() || trimmed_bytes[0] == b'-' {
            return None;
        }

        // Count whitespace-separated parts without allocating
        let mut parts = trimmed.split_whitespace();
        let name = parts.next()?;

        // Need at least 2 more words for description (total 3+)
        let second = parts.next()?;
        let third = parts.next();

        third?;

        if !Self::is_valid_subcommand_name(name) {
            return None;
        }

        // Build description from remaining parts
        let mut desc = EcoString::from(second);
        desc.push(' ');
        desc.push_str(third.unwrap());
        for part in parts {
            desc.push(' ');
            desc.push_str(part);
        }

        Some(Subcommand {
            cmd: EcoString::from(name),
            desc,
        })
    }

    #[inline]
    fn is_valid_subcommand_name(name: &str) -> bool {
        let bytes = name.as_bytes();

        // Fast path: check first byte
        if bytes.is_empty() || bytes[0] == b'-' {
            return false;
        }

        // SIMD-friendly byte iteration
        bytes
            .iter()
            .all(|&b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_subcommands() {
        let content = "run       Run a command\nbuild     Build a project";
        let subs = SubcommandParser::parse(content);
        assert!(subs.iter().any(|s| s.cmd.as_str() == "run"));
        assert!(subs.iter().any(|s| s.cmd.as_str() == "build"));
    }

    #[test]
    fn test_is_valid_subcommand_name() {
        assert!(SubcommandParser::is_valid_subcommand_name("run"));
        assert!(SubcommandParser::is_valid_subcommand_name("sub-cmd"));
        assert!(!SubcommandParser::is_valid_subcommand_name("-v"));
        assert!(!SubcommandParser::is_valid_subcommand_name(""));
    }
}
