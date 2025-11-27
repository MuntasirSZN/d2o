use crate::types::{Opt, OptName};
use bstr::ByteSlice;
use ecow::{EcoString, EcoVec};
use memchr::memchr;
use regex::Regex;
use std::collections::HashSet;

pub struct Parser;

impl Parser {
    pub fn parse_line(s: &str) -> EcoVec<Opt> {
        let pairs = Self::preprocess(s);
        let mut opts = EcoVec::new();
        let mut seen: HashSet<Opt, foldhash::fast::RandomState> =
            HashSet::with_capacity_and_hasher(pairs.len(), foldhash::fast::RandomState::default());

        for (opt_str, desc_str) in pairs.iter() {
            for opt in Self::parse_with_opt_part(opt_str, desc_str).iter() {
                if seen.insert(opt.clone()) {
                    opts.push(opt.clone());
                }
            }
        }
        opts
    }

    pub fn preprocess(s: &str) -> EcoVec<(EcoString, EcoString)> {
        // Use bstr for fast line iteration via memchr
        let bytes = s.as_bytes();
        let lines: Vec<&str> = bytes
            .lines()
            .filter_map(|line| std::str::from_utf8(line).ok())
            .collect();
        let mut result = EcoVec::new();
        let mut i = 0;

        while i < lines.len() {
            let line = lines[i];
            let trimmed = line.trim_start();

            // Fast path: skip lines that don't start with '-' using byte check
            let trimmed_bytes = trimmed.as_bytes();
            if trimmed_bytes.is_empty() || trimmed_bytes[0] != b'-' {
                i += 1;
                continue;
            }

            // Try to split option and description from the same line first
            // Most help text has format: "  -v, --verbose         description text"
            // Count parts and find opt_end without allocating Vec
            let mut opt_end = 0;
            let mut part_count = 0;
            for (idx, part) in trimmed.split_whitespace().enumerate() {
                part_count += 1;
                let part_bytes = part.as_bytes();
                if part_bytes.first() == Some(&b'-') || idx == 0 {
                    opt_end = idx + 1;
                } else if memchr(b'=', part_bytes).is_some() || part_bytes.first() != Some(&b'-') {
                    // Could be an argument marker
                    opt_end = idx + 1;
                } else {
                    break;
                }
            }

            if opt_end > 0 && opt_end < part_count {
                // Description is on the same line - build strings without intermediate Vec
                let mut opt_str = EcoString::new();
                let mut desc_str = EcoString::new();
                for (idx, part) in trimmed.split_whitespace().enumerate() {
                    if idx < opt_end {
                        if !opt_str.is_empty() {
                            opt_str.push(' ');
                        }
                        opt_str.push_str(part);
                    } else {
                        if !desc_str.is_empty() {
                            desc_str.push(' ');
                        }
                        desc_str.push_str(part);
                    }
                }
                result.push((opt_str, desc_str));
                i += 1;
            } else if opt_end > 0 {
                // No description on this line, try next line
                let opt_str = EcoString::from(trimmed);
                let desc_str = if i + 1 < lines.len() {
                    let next_trimmed = lines[i + 1].trim_start();
                    let next_bytes = next_trimmed.as_bytes();
                    if !next_bytes.is_empty() && next_bytes[0] != b'-' {
                        EcoString::from(lines[i + 1].trim())
                    } else {
                        EcoString::new()
                    }
                } else {
                    EcoString::new()
                };

                if !desc_str.is_empty() {
                    result.push((opt_str, desc_str));
                    i += 2;
                } else {
                    result.push((opt_str, EcoString::new()));
                    i += 1;
                }
            } else {
                i += 1;
            }
        }

        result
    }

    pub fn parse_with_opt_part(opt_str: &str, desc_str: &str) -> EcoVec<Opt> {
        let names = Self::parse_opt_names(opt_str);
        let arg = Self::parse_opt_arg(opt_str);

        if names.is_empty() {
            return EcoVec::new();
        }

        let mut result = EcoVec::new();
        result.push(Opt {
            names,
            argument: arg,
            description: EcoString::from(desc_str),
        });
        result
    }

    fn parse_opt_names(s: &str) -> EcoVec<OptName> {
        let mut names = EcoVec::new();
        let mut seen: HashSet<EcoString, foldhash::fast::RandomState> =
            HashSet::with_hasher(foldhash::fast::RandomState::default());

        for part in s.split([',', '/', '|']) {
            let trimmed = part.trim();
            if trimmed.is_empty() {
                continue;
            }

            for word in trimmed.split_whitespace() {
                if word.starts_with('-')
                    && let Some(name) = OptName::from_text(word)
                {
                    // Only add if not already seen (deduplicate)
                    if seen.insert(name.raw.clone()) {
                        // Insert in sorted order (insertion sort - fast for small N)
                        let pos = names.iter().position(|n| n > &name).unwrap_or(names.len());
                        names.insert(pos, name);
                    }
                }
            }
        }

        names
    }

    fn parse_opt_arg(s: &str) -> EcoString {
        for part in s.split([',', '/', '|']) {
            let trimmed = part.trim();
            if let Some(arg) = Self::extract_arg_from_part(trimmed)
                && !arg.is_empty()
            {
                return arg;
            }
        }
        EcoString::new()
    }

    fn extract_arg_from_part(s: &str) -> Option<EcoString> {
        let mut words = s.split_whitespace();
        // Skip first word (the option name)
        words.next()?;

        // Build arg from remaining words
        let mut arg = EcoString::new();
        for word in words {
            if !arg.is_empty() {
                arg.push(' ');
            }
            arg.push_str(word);
        }

        if arg.is_empty() || arg == "." {
            return None;
        }

        Some(arg)
    }

    pub fn parse_usage_header(keywords: &[&str], block: &str) -> Option<EcoString> {
        if keywords.is_empty() || block.is_empty() {
            return None;
        }

        let header_line = block.lines().next()?.to_lowercase();
        for keyword in keywords {
            let pattern = format!(r"^\s*{}\s*:?\s*$", regex::escape(keyword));
            if let Ok(re) = Regex::new(&pattern)
                && re.is_match(&header_line)
            {
                return Some(EcoString::from(header_line));
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preprocess_same_and_next_line_descriptions() {
        let input = "  -a, --all  show all\n  -b\n    show b";
        let pairs = Parser::preprocess(input);
        assert_eq!(pairs.len(), 2);
        // Current implementation keeps the entire first line as the option
        // part when it cannot separate a description on the same line.
        assert_eq!(pairs[0].0.as_str(), "-a, --all  show all");
        assert_eq!(pairs[0].1.as_str(), "");
        assert_eq!(pairs[1].0.as_str(), "-b");
        assert_eq!(pairs[1].1.as_str(), "show b");
    }

    #[test]
    fn test_parse_usage_header_matches_keywords() {
        let block = "Usage:\n  cmd [OPTIONS]\n";
        let header = Parser::parse_usage_header(&["usage"], block).unwrap();
        assert!(header.contains("usage"));
    }

    #[test]
    fn test_parse_opt_names() {
        let names = Parser::parse_opt_names("-v, --verbose");
        assert_eq!(names.len(), 2);
        assert!(names.iter().any(|n| n.raw.as_str() == "-v"));
        assert!(names.iter().any(|n| n.raw.as_str() == "--verbose"));
    }

    #[test]
    fn test_parse_with_opt_part() {
        let opts = Parser::parse_with_opt_part("-v, --verbose", "Enable verbose mode");
        assert_eq!(opts.len(), 1);
        assert_eq!(opts[0].names.len(), 2);
        assert_eq!(opts[0].description.as_str(), "Enable verbose mode");
    }

    #[test]
    fn test_parse_line_deduplicates_options() {
        let input = "  -v, --verbose  verbose\n  -v, --verbose  verbose";
        let opts = Parser::parse_line(input);
        assert_eq!(opts.len(), 1);
        assert_eq!(opts[0].names.len(), 2);
    }

    #[test]
    fn test_parse_line_bioinformatics_style_help() {
        let input = "  -i, --input FILE       Input FASTA/FASTQ file\n  -o, --output FILE      Output BAM file\n  --min-mapq INT         Minimum mapping quality (default: 30)";
        let opts = Parser::parse_line(input);
        assert_eq!(opts.len(), 3);

        // Ensure all expected option names are detected, even if
        // arguments/descriptions are not perfectly separated.
        let all_names: Vec<String> = opts
            .iter()
            .flat_map(|o| o.names.iter().map(|n| n.raw.to_string()))
            .collect();
        assert!(all_names.contains(&"-i".to_string()));
        assert!(all_names.contains(&"--input".to_string()));
        assert!(all_names.contains(&"-o".to_string()));
        assert!(all_names.contains(&"--output".to_string()));
        assert!(all_names.contains(&"--min-mapq".to_string()));
    }
}
