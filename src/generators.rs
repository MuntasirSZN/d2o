use crate::types::{Command, Opt, OptName, OptNameType};
use aho_corasick::AhoCorasick;
use ecow::EcoString;
use memchr::memchr;
use std::collections::BTreeSet;
use std::fmt::Write;
use std::sync::LazyLock;

// Pre-compiled Aho-Corasick automaton for file/dir/path matching (SIMD-accelerated)
static FILE_PATH_MATCHER: LazyLock<AhoCorasick> = LazyLock::new(|| {
    AhoCorasick::builder()
        .ascii_case_insensitive(true)
        .build(["file", "dir", "path", "archive"])
        .unwrap()
});

pub struct FishGenerator;

impl FishGenerator {
    pub fn generate(cmd: &Command) -> EcoString {
        // Pre-calculate capacity based on options count
        let estimated_size = 64 + cmd.options.len() * 80;
        let mut buf = String::with_capacity(estimated_size);
        Self::generate_rec(&mut buf, &[], cmd);
        // Remove trailing newline if present
        if buf.ends_with('\n') {
            buf.pop();
        }
        EcoString::from(buf)
    }

    fn generate_rec(buf: &mut String, path: &[&str], cmd: &Command) {
        let mut current_path = path.to_vec();
        current_path.push(&cmd.name);
        let path_str = current_path.join("_");

        for opt in cmd.options.iter() {
            for name in opt.names.iter() {
                if !Self::should_skip_option(name) {
                    Self::write_option_line(buf, &path_str, name, opt);
                }
            }
        }

        for subcmd in cmd.subcommands.iter() {
            Self::generate_rec(buf, &current_path, subcmd);
        }
    }

    #[inline]
    fn should_skip_option(name: &OptName) -> bool {
        matches!(
            name.opt_type,
            OptNameType::SingleDashAlone | OptNameType::DoubleDashAlone
        )
    }

    fn write_option_line(buf: &mut String, path_str: &str, name: &OptName, opt: &Opt) {
        let dashless = name.raw.trim_start_matches('-');
        let flag = Self::opt_type_to_flag(name.opt_type);
        let arg_flag = Self::opt_arg_to_flag(opt);
        let desc = Self::truncate_after_period(&opt.description);

        let _ = writeln!(
            buf,
            "complete -c {} {} '{}' {} -d '{}'",
            path_str,
            flag,
            dashless,
            arg_flag,
            desc.replace('\'', "\\'")
        );
    }

    #[inline]
    fn opt_type_to_flag(opt_type: OptNameType) -> &'static str {
        match opt_type {
            OptNameType::LongType => "-l",
            OptNameType::ShortType => "-s",
            OptNameType::OldType => "-o",
            _ => "",
        }
    }

    /// Use Aho-Corasick automaton for SIMD-accelerated multi-pattern matching
    #[inline]
    fn opt_arg_to_flag(opt: &Opt) -> &'static str {
        if opt.argument.is_empty() {
            return "";
        }

        // Use pre-compiled Aho-Corasick for SIMD multi-pattern search
        if FILE_PATH_MATCHER.is_match(opt.argument.as_str()) {
            return "-r";
        }

        if FILE_PATH_MATCHER.is_match(opt.description.as_str()) {
            return "-r";
        }

        "-x"
    }

    /// Truncate string after first period using SIMD-accelerated memchr
    #[inline]
    pub fn truncate_after_period(line: &str) -> &str {
        // Use memchr for SIMD-accelerated '.' search
        match memchr(b'.', line.as_bytes()) {
            Some(pos) => &line[..pos],
            None => line,
        }
    }
}

pub struct ZshGenerator;

impl ZshGenerator {
    pub fn generate(cmd: &Command) -> EcoString {
        let estimated_size = 256 + cmd.options.len() * 64;
        let mut buf = String::with_capacity(estimated_size);

        let _ = writeln!(buf, "#compdef {}", cmd.name);
        let _ = writeln!(buf);
        let _ = writeln!(buf, "_{}() {{", cmd.name);
        let _ = writeln!(buf, "  local -a options");
        let _ = writeln!(buf);

        for opt in cmd.options.iter() {
            Self::write_opt(&mut buf, opt);
        }

        let _ = writeln!(buf, "  _arguments -s -S $options");
        let _ = writeln!(buf, "}}");
        let _ = writeln!(buf);
        let _ = write!(buf, "_{} \"$@\"", cmd.name);

        EcoString::from(buf)
    }

    fn write_opt(buf: &mut String, opt: &Opt) {
        let desc = FishGenerator::truncate_after_period(&opt.description);

        for name in opt.names.iter() {
            if matches!(
                name.opt_type,
                OptNameType::SingleDashAlone | OptNameType::DoubleDashAlone
            ) {
                continue;
            }

            if opt.argument.is_empty() {
                let _ = writeln!(buf, "  options+=('{}[{}]')", name.raw, desc);
            } else {
                let _ = writeln!(
                    buf,
                    "  options+=('{}[{} {}]')",
                    name.raw, opt.argument, desc
                );
            }
        }
    }
}

pub struct BashGenerator;

impl BashGenerator {
    pub fn generate(cmd: &Command) -> EcoString {
        Self::generate_with_compat(cmd, false)
    }

    pub fn generate_with_compat(cmd: &Command, bash_completion_compat: bool) -> EcoString {
        let estimated_size = 512 + cmd.options.len() * 32;
        let mut buf = String::with_capacity(estimated_size);

        let _ = writeln!(buf, "_{}()", cmd.name);
        let _ = writeln!(buf, "{{");
        let _ = writeln!(buf, "  local cur prev opts");
        let _ = writeln!(buf, "  COMPREPLY=()");
        let _ = writeln!(buf, "  cur=\"${{COMP_WORDS[COMP_CWORD]}}\"");
        let _ = writeln!(buf, "  prev=\"${{COMP_WORDS[COMP_CWORD-1]}}\"");
        let _ = writeln!(buf);

        // Collect all option strings into a BTreeSet for deduplication and sorting
        let all_opts: BTreeSet<String> = if bash_completion_compat {
            cmd.options
                .iter()
                .flat_map(|opt| {
                    let base_desc = FishGenerator::truncate_after_period(&opt.description);
                    let desc: String = base_desc
                        .split_whitespace()
                        .collect::<Vec<_>>()
                        .join("_")
                        .replace(':', "_");

                    opt.names
                        .iter()
                        .filter_map(|name| {
                            if matches!(
                                name.opt_type,
                                OptNameType::SingleDashAlone | OptNameType::DoubleDashAlone
                            ) {
                                None
                            } else if desc.is_empty() {
                                Some(name.raw.to_string())
                            } else {
                                let mut s = String::with_capacity(name.raw.len() + desc.len() + 1);
                                s.push_str(&name.raw);
                                s.push(':');
                                s.push_str(&desc);
                                Some(s)
                            }
                        })
                        .collect::<Vec<_>>()
                })
                .collect()
        } else {
            cmd.options
                .iter()
                .flat_map(|opt| {
                    opt.names
                        .iter()
                        .filter_map(|name| {
                            if matches!(
                                name.opt_type,
                                OptNameType::SingleDashAlone | OptNameType::DoubleDashAlone
                            ) {
                                None
                            } else {
                                Some(name.raw.to_string())
                            }
                        })
                        .collect::<Vec<_>>()
                })
                .collect()
        };

        // Build opts string efficiently
        let opts_joined = all_opts.into_iter().collect::<Vec<_>>().join(" ");
        let _ = writeln!(buf, "  opts=\"{}\"", opts_joined);
        let _ = writeln!(buf);
        let _ = writeln!(buf, "  COMPREPLY=($(compgen -W \"${{opts}}\" -- ${{cur}}))");

        if bash_completion_compat {
            let _ = writeln!(buf, "  if type __ltrim_colon_completions &>/dev/null; then");
            let _ = writeln!(buf, "    __ltrim_colon_completions \"$cur\"");
            let _ = writeln!(buf, "  fi");
        }

        let _ = writeln!(buf, "}}");
        let _ = writeln!(buf);
        let _ = write!(
            buf,
            "complete -o bashdefault -o default -o nospace -F _{} {}",
            cmd.name, cmd.name
        );

        EcoString::from(buf)
    }
}

pub struct ElvishGenerator;

impl ElvishGenerator {
    pub fn generate(cmd: &Command) -> EcoString {
        let estimated_size = 512 + cmd.options.len() * 48;
        let mut buf = String::with_capacity(estimated_size);

        let _ = writeln!(buf, "use builtin;");
        let _ = writeln!(buf, "use str;");
        let _ = writeln!(buf);
        let _ = writeln!(
            buf,
            "set edit:completion:arg-completer[{}] = {{|@words|",
            cmd.name
        );
        let _ = writeln!(buf, "    fn spaces {{|n|");
        let _ = writeln!(buf, "        builtin:repeat $n ' ' | str:join ''");
        let _ = writeln!(buf, "    }}");
        let _ = writeln!(buf, "    fn cand {{|text desc|");
        let _ = writeln!(
            buf,
            "        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc"
        );
        let _ = writeln!(buf, "    }}");
        let _ = writeln!(buf, "    var command = '{}'", cmd.name);
        let _ = writeln!(buf, "    for word $words[1..-1] {{");
        let _ = writeln!(buf, "        if (str:has-prefix $word '-') {{");
        let _ = writeln!(buf, "            break");
        let _ = writeln!(buf, "        }}");
        let _ = writeln!(buf, "        set command = $command';'$word");
        let _ = writeln!(buf, "    }}");
        let _ = writeln!(buf, "    var completions = [");
        let _ = writeln!(buf, "        &'{}'= {{", cmd.name);

        for opt in cmd.options.iter() {
            let desc = FishGenerator::truncate_after_period(&opt.description);
            let desc_clean = desc.replace('\'', "");
            for name in opt.names.iter() {
                if matches!(
                    name.opt_type,
                    OptNameType::SingleDashAlone | OptNameType::DoubleDashAlone
                ) {
                    continue;
                }
                let _ = writeln!(buf, "            cand {} '{}'", name.raw, desc_clean);
            }
        }

        let _ = writeln!(buf, "        }}");
        let _ = writeln!(buf, "    ]");
        let _ = writeln!(buf, "    $completions[$command]");
        let _ = write!(buf, "}}");

        EcoString::from(buf)
    }
}

pub struct NushellGenerator;

impl NushellGenerator {
    pub fn generate(cmd: &Command) -> EcoString {
        let estimated_size = 512 + cmd.options.len() * 48;
        let mut buf = String::with_capacity(estimated_size);

        let _ = writeln!(buf, "module completions {{");
        let _ = writeln!(buf);
        let _ = writeln!(buf, "  # Completions for {} options", cmd.name);
        let _ = writeln!(buf, "  def \"nu-complete {} options\" [] {{", cmd.name);

        // Collect options into BTreeSet for deduplication and sorting
        let all_opts: BTreeSet<&str> = cmd
            .options
            .iter()
            .flat_map(|opt| {
                opt.names
                    .iter()
                    .filter_map(|name| {
                        if !matches!(
                            name.opt_type,
                            OptNameType::SingleDashAlone | OptNameType::DoubleDashAlone
                        ) {
                            Some(name.raw.as_str())
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
            })
            .collect();

        if all_opts.is_empty() {
            let _ = writeln!(buf, "    []");
        } else {
            let _ = write!(buf, "    [ ");
            for (i, opt) in all_opts.iter().enumerate() {
                if i > 0 {
                    let _ = write!(buf, " ");
                }
                let _ = write!(buf, "\"{}\"", opt);
            }
            let _ = writeln!(buf, " ]");
        }
        let _ = writeln!(buf, "  }}");
        let _ = writeln!(buf);

        let _ = writeln!(buf, "  export extern {} [", cmd.name);

        for opt in cmd.options.iter() {
            let desc = FishGenerator::truncate_after_period(&opt.description);

            for name in opt.names.iter() {
                if matches!(
                    name.opt_type,
                    OptNameType::SingleDashAlone | OptNameType::DoubleDashAlone
                ) {
                    continue;
                }

                if opt.argument.is_empty() {
                    let _ = writeln!(buf, "    {} # {}", name.raw, desc);
                } else {
                    let _ = writeln!(
                        buf,
                        "    {}: string  # {} # {}",
                        name.raw, opt.argument, desc
                    );
                }
            }
        }

        let _ = writeln!(buf, "  ]");
        let _ = writeln!(buf);
        let _ = writeln!(buf, "}}");
        let _ = writeln!(buf);
        let _ = write!(buf, "export use completions *");

        EcoString::from(buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_after_period() {
        let text = "This is a description. With more text.";
        assert_eq!(
            FishGenerator::truncate_after_period(text),
            "This is a description"
        );
    }
}
