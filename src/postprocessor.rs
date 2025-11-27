use crate::types::{Command, Opt, OptName};
use bstr::ByteSlice;
use ecow::{EcoString, EcoVec};
use memchr::memchr;
use std::collections::HashSet;

pub struct Postprocessor;

impl Postprocessor {
    pub fn fix_command(mut cmd: Command) -> Command {
        cmd.options = Self::deduplicate_options(cmd.options);
        cmd.options = Self::filter_invalid_options(cmd.options);
        cmd.subcommands = cmd.subcommands.into_iter().map(Self::fix_command).collect();

        cmd
    }

    fn deduplicate_options(options: EcoVec<Opt>) -> EcoVec<Opt> {
        // Deduplicate based on (names, argument) - description is not part of the key
        let mut seen: HashSet<(EcoVec<OptName>, EcoString), foldhash::fast::RandomState> =
            HashSet::with_capacity_and_hasher(
                options.len(),
                foldhash::fast::RandomState::default(),
            );
        let mut result = EcoVec::new();

        for opt in options.iter() {
            let key = (opt.names.clone(), opt.argument.clone());
            if seen.insert(key) {
                result.push(opt.clone());
            }
        }

        result
    }

    fn filter_invalid_options(options: EcoVec<Opt>) -> EcoVec<Opt> {
        options
            .into_iter()
            .filter(|opt| {
                !opt.names.is_empty() && !opt.names[0].raw.is_empty() && !opt.description.is_empty()
            })
            .collect()
    }

    pub fn remove_bullets(text: &str) -> EcoString {
        let bytes = text.as_bytes();

        // SIMD fast path: check if any bullet characters exist
        // Bullets we care about: '*' (0x2A), '-' (0x2D), '•' (0xE2 0x80 0xA2)
        let has_asterisk = memchr(b'*', bytes).is_some();
        let has_dash = memchr(b'-', bytes).is_some();
        let has_bullet_utf8 = memchr(0xE2, bytes).is_some();

        if !has_asterisk && !has_dash && !has_bullet_utf8 {
            return EcoString::from(text);
        }

        // Pre-allocate with capacity hint
        let mut result = String::with_capacity(text.len());
        let mut first = true;

        // Use bstr for fast line iteration (SIMD-accelerated newline search)
        for line in bytes.lines() {
            if !first {
                result.push('\n');
            }
            first = false;

            // Safe: we know bytes came from valid UTF-8
            let line_str = unsafe { std::str::from_utf8_unchecked(line) };
            let trimmed = line_str.trim_start();
            let prefix_len = line_str.len() - trimmed.len();
            let trimmed_bytes = trimmed.as_bytes();

            // Fast path: check first byte for bullet characters
            if trimmed_bytes.len() >= 2 {
                let is_bullet = match trimmed_bytes[0] {
                    b'*' | b'-' => trimmed_bytes[1].is_ascii_whitespace(),
                    // UTF-8 bullet point (•) starts with 0xE2
                    0xE2 if trimmed_bytes.len() >= 4
                        && trimmed_bytes[1] == 0x80
                        && trimmed_bytes[2] == 0xA2 =>
                    {
                        trimmed_bytes[3].is_ascii_whitespace()
                    }
                    _ => false,
                };

                if is_bullet {
                    result.push_str(&line_str[..prefix_len]);
                    // Skip bullet and whitespace
                    let skip = if trimmed_bytes[0] == 0xE2 { 4 } else { 2 };
                    result.push_str(trimmed[skip..].trim_start());
                    continue;
                }
            }
            result.push_str(line_str);
        }

        EcoString::from(result)
    }

    pub fn unicode_spaces_to_ascii(text: &str) -> EcoString {
        let bytes = text.as_bytes();

        // SIMD fast path: scan for any high bytes using memchr
        // All unicode spaces we care about have bytes >= 0x80
        // Use memchr to find first high byte - this is SIMD accelerated
        if memchr::memchr(0x80, bytes).is_none()
            && memchr::memchr(0xC2, bytes).is_none()
            && memchr::memchr(0xE2, bytes).is_none()
        {
            // Pure ASCII - no unicode spaces possible
            return EcoString::from(text);
        }

        // Check if any of our target characters exist using a single pass
        let has_targets = text.chars().any(|c| {
            matches!(
                c,
                '\u{00A0}' | '\u{2002}' | '\u{2003}' | '\u{2009}' | '\u{202F}'
            )
        });

        if !has_targets {
            return EcoString::from(text);
        }

        // Pre-allocate result
        let mut result = String::with_capacity(text.len() + text.len() / 8);

        for c in text.chars() {
            match c {
                '\u{00A0}' | '\u{202F}' => result.push(' '), // NBSP, Narrow NBSP
                '\u{2009}' => result.push(' '),              // Thin space
                '\u{2002}' => result.push_str("  "),         // En space
                '\u{2003}' => result.push_str("   "),        // Em space
                _ => result.push(c),
            }
        }

        EcoString::from(result)
    }

    pub fn convert_tabs_to_spaces(text: &str, spaces: usize) -> EcoString {
        // SIMD fast path: use memchr to check for tabs
        if memchr(b'\t', text.as_bytes()).is_none() {
            return EcoString::from(text);
        }
        EcoString::from(text.replace('\t', &" ".repeat(spaces)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::OptName;
    use crate::types::OptNameType;
    use ecow::EcoString;

    #[test]
    fn test_deduplicate_options() {
        let mut opts = EcoVec::new();
        opts.push(Opt {
            names: {
                let mut v = EcoVec::new();
                v.push(OptName::new(EcoString::from("-v"), OptNameType::ShortType));
                v
            },
            argument: EcoString::new(),
            description: EcoString::from("verbose"),
        });
        opts.push(Opt {
            names: {
                let mut v = EcoVec::new();
                v.push(OptName::new(EcoString::from("-v"), OptNameType::ShortType));
                v
            },
            argument: EcoString::new(),
            description: EcoString::from("verbose"),
        });

        let result = Postprocessor::deduplicate_options(opts);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_remove_bullets() {
        let text = "• Item one\n* Item two\n- Item three";
        let result = Postprocessor::remove_bullets(text);
        assert!(!result.contains("•"));
    }

    #[test]
    fn test_unicode_and_tabs_helpers() {
        // Text with various unicode spaces and a tab
        let text = "\u{00A0}foo\u{2002}bar\u{2003}baz\tend";
        let ascii = Postprocessor::unicode_spaces_to_ascii(text);

        // Non-breaking/en-space/em-space should be replaced with ASCII spaces
        assert_eq!(ascii.as_str(), " foo  bar   baz\tend");

        let with_spaces = Postprocessor::convert_tabs_to_spaces(&ascii, 4);
        assert!(!with_spaces.contains('\t'));
        assert!(with_spaces.ends_with("    end"));
    }

    #[test]
    fn test_fix_command_filters_and_deduplicates() {
        let valid_opt = Opt {
            names: {
                let mut v = EcoVec::new();
                v.push(OptName::new(EcoString::from("-v"), OptNameType::ShortType));
                v
            },
            argument: EcoString::new(),
            description: EcoString::from("verbose"),
        };

        let invalid_opt = Opt {
            names: EcoVec::new(),
            argument: EcoString::new(),
            description: EcoString::new(),
        };

        let cmd = Command {
            name: EcoString::from("root"),
            description: EcoString::new(),
            usage: EcoString::new(),
            options: {
                let mut v = EcoVec::new();
                v.push(valid_opt.clone());
                v.push(valid_opt.clone());
                v.push(invalid_opt);
                v
            },
            subcommands: {
                let mut v = EcoVec::new();
                v.push(Command {
                    name: EcoString::from("child"),
                    description: EcoString::new(),
                    usage: EcoString::new(),
                    options: {
                        let mut opts = EcoVec::new();
                        opts.push(valid_opt.clone());
                        opts
                    },
                    subcommands: EcoVec::new(),
                    version: EcoString::new(),
                });
                v
            },
            version: EcoString::new(),
        };

        let fixed = Postprocessor::fix_command(cmd);
        assert_eq!(fixed.options.len(), 1);
        assert_eq!(fixed.subcommands.len(), 1);
        assert_eq!(fixed.subcommands[0].options.len(), 1);
    }
}
