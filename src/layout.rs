use crate::parser::Parser;
use crate::types::Opt;
use rayon::prelude::*;
use std::collections::HashMap;

pub struct Layout;

impl Layout {
    /// Parse content into options, processing blocks in parallel.
    pub fn parse_blockwise(content: &str) -> Vec<Opt> {
        let blocks = Self::split_into_blocks(content);

        // Use parallel iterator for processing multiple blocks
        blocks
            .par_iter()
            .flat_map(|block| Parser::parse_line(block))
            .collect()
    }

    /// Preprocess content into option/description pairs, processing blocks in parallel.
    pub fn preprocess_blockwise(content: &str) -> Vec<(String, String)> {
        let blocks = Self::split_into_blocks(content);

        // Use parallel iterator for processing multiple blocks
        blocks
            .par_iter()
            .flat_map(|block| Parser::preprocess(block))
            .collect()
    }

    pub fn parse_usage(content: &str) -> String {
        let keywords = ["usage", "synopsis"];
        let lines: Vec<&str> = content.lines().collect();

        for (i, line) in lines.iter().enumerate() {
            let lower = line.to_lowercase();
            if keywords.iter().any(|k| lower.contains(k)) && lower.contains(':') {
                let usage_lines: Vec<&str> = lines[i..]
                    .iter()
                    .take_while(|l| !l.is_empty() && (l.starts_with(' ') || l.contains(':')))
                    .copied()
                    .collect();

                if !usage_lines.is_empty() {
                    return usage_lines.join("\n");
                }
            }
        }

        String::new()
    }

    fn split_into_blocks(content: &str) -> Vec<String> {
        let lines: Vec<&str> = content.lines().collect();
        let mut blocks = Vec::new();
        let mut current_block = Vec::new();

        for line in lines {
            let trimmed = line.trim_start();

            if trimmed.is_empty() {
                if !current_block.is_empty() {
                    blocks.push(current_block.join("\n"));
                    current_block = Vec::new();
                }
            } else if trimmed.starts_with('-') || !current_block.is_empty() {
                current_block.push(line);
            }
        }

        if !current_block.is_empty() {
            blocks.push(current_block.join("\n"));
        }

        blocks
    }

    pub fn get_option_offsets(s: &str) -> Vec<usize> {
        let short_offset = Self::get_short_option_offset(s);
        let long_offset = Self::get_long_option_offset(s);

        match (short_offset, long_offset) {
            (None, None) => Vec::new(),
            (None, Some(y)) => vec![y],
            (Some(x), None) => vec![x],
            (Some(x), Some(y)) => {
                if x == y {
                    vec![x]
                } else {
                    vec![x, y]
                }
            }
        }
    }

    fn get_option_locations(s: &str, predicate: fn(&str) -> bool) -> Vec<(usize, usize)> {
        s.lines()
            .enumerate()
            .filter_map(|(i, line)| {
                let trimmed = line.trim_start();
                if !trimmed.is_empty() && predicate(trimmed) {
                    let offset = line.len() - trimmed.len();
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

        let mut freq_map = HashMap::new();
        for (_, offset) in locations {
            *freq_map.entry(*offset).or_insert(0) += 1;
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
