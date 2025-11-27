use crate::parser::Parser;
use crate::types::Opt;
use bstr::ByteSlice;
use ecow::{EcoString, EcoVec};
use memchr::memchr;
use rayon::prelude::*;

pub struct Layout;

impl Layout {
    /// Parse content into options, processing blocks in parallel.
    pub fn parse_blockwise(content: &str) -> EcoVec<Opt> {
        let blocks = Self::split_into_blocks_fast(content);

        // Use parallel iterator for processing multiple blocks
        // Only parallelize if we have enough blocks to benefit
        if blocks.len() > 4 {
            blocks
                .par_iter()
                .flat_map(|block| {
                    let opts = Parser::parse_line(block);
                    opts.into_iter().collect::<Vec<_>>()
                })
                .collect::<Vec<_>>()
                .into_iter()
                .collect()
        } else {
            blocks
                .iter()
                .flat_map(|block| Parser::parse_line(block).into_iter())
                .collect()
        }
    }

    /// Preprocess content into option/description pairs, processing blocks in parallel.
    pub fn preprocess_blockwise(content: &str) -> EcoVec<(EcoString, EcoString)> {
        let blocks = Self::split_into_blocks_fast(content);

        // Only parallelize if we have enough blocks
        if blocks.len() > 4 {
            blocks
                .par_iter()
                .flat_map(|block| {
                    let pairs = Parser::preprocess(block);
                    pairs.into_iter().collect::<Vec<_>>()
                })
                .collect::<Vec<_>>()
                .into_iter()
                .collect()
        } else {
            blocks
                .iter()
                .flat_map(|block| Parser::preprocess(block).into_iter())
                .collect()
        }
    }

    pub fn parse_usage(content: &str) -> EcoString {
        let keywords = ["usage", "synopsis"];
        let bytes = content.as_bytes();

        // SIMD fast scan for 'u' or 's' (first chars of keywords)
        if memchr(b'u', bytes).is_none() && memchr(b's', bytes).is_none() {
            // Also check uppercase
            if memchr(b'U', bytes).is_none() && memchr(b'S', bytes).is_none() {
                return EcoString::new();
            }
        }

        // Fast scan for keywords first
        let lower = content.to_lowercase();
        let mut keyword_pos = None;
        for keyword in &keywords {
            if let Some(pos) = lower.find(keyword) {
                // Check if followed by ':'
                let rest = &lower[pos..];
                if rest.contains(':') {
                    keyword_pos = Some(pos);
                    break;
                }
            }
        }

        if keyword_pos.is_none() {
            return EcoString::new();
        }

        // Use bstr for SIMD-accelerated line iteration
        let lines: Vec<&str> = bytes
            .lines()
            .filter_map(|line| std::str::from_utf8(line).ok())
            .collect();

        for (i, line) in lines.iter().enumerate() {
            let lower = line.to_lowercase();
            if keywords.iter().any(|k| lower.contains(k)) && lower.contains(':') {
                let mut usage_result = String::with_capacity(256);
                let mut first = true;

                for l in lines[i..].iter() {
                    if (l.is_empty() || (!l.starts_with(' ') && !l.contains(':')))
                        && !first {
                            break;
                        }
                    if !first {
                        usage_result.push('\n');
                    }
                    usage_result.push_str(l);
                    first = false;
                }

                if !usage_result.is_empty() {
                    return EcoString::from(usage_result);
                }
            }
        }

        EcoString::new()
    }

    /// Optimized block splitting that minimizes allocations
    /// Uses bstr for SIMD-accelerated line iteration
    fn split_into_blocks_fast(content: &str) -> EcoVec<EcoString> {
        let bytes = content.as_bytes();

        // SIMD fast path: check if '-' exists at all
        if memchr(b'-', bytes).is_none() {
            return EcoVec::new();
        }

        let mut blocks = EcoVec::new();
        let mut current_block = String::with_capacity(256);
        let mut in_block = false;

        // Use bstr for SIMD-accelerated line iteration
        for line in bytes.lines() {
            // Safe conversion - content is already valid UTF-8
            let line_str = unsafe { std::str::from_utf8_unchecked(line) };
            let trimmed = line_str.trim_start();

            if trimmed.is_empty() {
                if in_block && !current_block.is_empty() {
                    blocks.push(EcoString::from(current_block.as_str()));
                    current_block.clear();
                    in_block = false;
                }
            } else if trimmed.starts_with('-') || in_block {
                if !current_block.is_empty() {
                    current_block.push('\n');
                }
                current_block.push_str(line_str);
                in_block = true;
            }
        }

        if !current_block.is_empty() {
            blocks.push(EcoString::from(current_block));
        }

        blocks
    }

    pub fn get_option_offsets(s: &str) -> EcoVec<usize> {
        let short_offset = Self::get_short_option_offset(s);
        let long_offset = Self::get_long_option_offset(s);

        let mut result = EcoVec::new();
        match (short_offset, long_offset) {
            (None, None) => {}
            (None, Some(y)) => result.push(y),
            (Some(x), None) => result.push(x),
            (Some(x), Some(y)) => {
                if x == y {
                    result.push(x);
                } else {
                    result.push(x);
                    result.push(y);
                }
            }
        }
        result
    }

    fn get_option_locations(s: &str, predicate: fn(&str) -> bool) -> EcoVec<(usize, usize)> {
        let bytes = s.as_bytes();

        // Use bstr for SIMD-accelerated line iteration
        bytes
            .lines()
            .enumerate()
            .filter_map(|(i, line)| {
                let line_str = std::str::from_utf8(line).ok()?;
                let trimmed = line_str.trim_start();
                if !trimmed.is_empty() && predicate(trimmed) {
                    let offset = line_str.len() - trimmed.len();
                    Some((i, offset))
                } else {
                    None
                }
            })
            .collect()
    }

    fn get_long_option_offset(s: &str) -> Option<usize> {
        let locations = Self::get_option_locations(s, |line| line.starts_with("--"));
        Self::get_most_frequent_offset(&locations)
    }

    fn get_short_option_offset(s: &str) -> Option<usize> {
        let locations =
            Self::get_option_locations(s, |line| line.starts_with('-') && !line.starts_with("--"));
        Self::get_most_frequent_offset(&locations)
    }

    fn get_most_frequent_offset(locations: &[(usize, usize)]) -> Option<usize> {
        if locations.is_empty() {
            return None;
        }

        // Use a simple local HashMap instead of concurrent one for this single-threaded operation
        let mut freq_map = std::collections::HashMap::with_capacity(locations.len());
        for (_, offset) in locations {
            *freq_map.entry(*offset).or_insert(0usize) += 1;
        }

        freq_map
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(offset, _)| offset)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_usage() {
        let content = "usage: command [options]\n\ndescription";
        let usage = Layout::parse_usage(content);
        assert!(!usage.is_empty());
    }

    #[test]
    fn test_parse_and_preprocess_blockwise() {
        let content = "\
  -a, --all        show all\n\
\n\
      --verbose    be verbose\n";

        let opts = Layout::parse_blockwise(content);
        assert_eq!(opts.len(), 2);

        let pairs = Layout::preprocess_blockwise(content);
        assert!(pairs.iter().any(|(opt, _)| opt.contains("-a")));
        assert!(pairs.iter().any(|(opt, _)| opt.contains("--verbose")));
    }

    #[test]
    fn test_get_option_offsets() {
        let content = "\
      -a, --all        show all\n\
      --verbose        be verbose\n";

        let offsets = Layout::get_option_offsets(content);
        assert!(!offsets.is_empty());
        // both short and long options are aligned, so we should get a single offset
        assert_eq!(offsets.len(), 1);
    }
}
