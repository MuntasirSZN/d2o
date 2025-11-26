use crate::types::{Command, Opt, OptName, OptNameType};
use std::collections::BTreeSet;

pub struct FishGenerator;

impl FishGenerator {
    pub fn generate(cmd: &Command) -> String {
        let mut lines = Vec::new();
        Self::generate_rec(&mut lines, &[], cmd);
        lines.join("\n")
    }

    fn generate_rec(lines: &mut Vec<String>, path: &[&str], cmd: &Command) {
        let mut current_path = path.to_vec();
        current_path.push(&cmd.name);

        for opt in &cmd.options {
            for name in &opt.names {
                if !Self::should_skip_option(name) {
                    lines.push(Self::make_option_line(&current_path, name, opt));
                }
            }
        }

        for subcmd in &cmd.subcommands {
            Self::generate_rec(lines, &current_path, subcmd);
        }
    }

    fn should_skip_option(name: &OptName) -> bool {
        matches!(
            name.opt_type,
            OptNameType::SingleDashAlone | OptNameType::DoubleDashAlone
        )
    }

    fn make_option_line(path: &[&str], name: &OptName, opt: &Opt) -> String {
        let dashless = name.raw.trim_start_matches('-');
        let quoted = format!("'{}'", dashless);
        let flag = Self::opt_type_to_flag(name.opt_type);
        let arg_flag = Self::opt_arg_to_flag(opt);
        let desc = Self::truncate_after_period(&opt.description);

        format!(
            "complete -c {} {} {} {} -d '{}'",
            path.join("_"),
            flag,
            quoted,
            arg_flag,
            desc.replace('\'', "\\'")
        )
    }

    fn opt_type_to_flag(opt_type: OptNameType) -> String {
        match opt_type {
            OptNameType::LongType => "-l".to_string(),
            OptNameType::ShortType => "-s".to_string(),
            OptNameType::OldType => "-o".to_string(),
            _ => String::new(),
        }
    }

    fn opt_arg_to_flag(opt: &Opt) -> &'static str {
        let arg_lower = opt.argument.to_lowercase();
        let desc_lower = opt.description.to_lowercase();

        if opt.argument.is_empty() {
            ""
        } else if arg_lower.contains("file")
            || arg_lower.contains("dir")
            || arg_lower.contains("path")
            || arg_lower.contains("archive")
            || desc_lower.contains("file")
            || desc_lower.contains("dir")
            || desc_lower.contains("path")
        {
            "-r"
        } else {
            "-x"
        }
    }

    fn truncate_after_period(line: &str) -> String {
        line.split('.').next().unwrap_or("").to_string()
    }
}

pub struct ZshGenerator;

impl ZshGenerator {
    pub fn generate(cmd: &Command) -> String {
        let mut lines = vec![
            format!("#compdef {}", cmd.name),
            "".to_string(),
            format!("_{}() {{", cmd.name),
            "  local -a options".to_string(),
            "".to_string(),
        ];

        for opt in &cmd.options {
            let opt_lines = Self::generate_opt(opt);
            lines.extend(opt_lines);
        }

        lines.push("  _arguments -s -S $options".to_string());
        lines.push("}".to_string());
        lines.push("".to_string());
        lines.push(format!("_{} \"$@\"", cmd.name));

        lines.join("\n")
    }

    fn generate_opt(opt: &Opt) -> Vec<String> {
        let mut lines = Vec::new();
        let desc = Self::truncate_after_period(&opt.description);

        for name in &opt.names {
            if matches!(
                name.opt_type,
                OptNameType::SingleDashAlone | OptNameType::DoubleDashAlone
            ) {
                continue;
            }

            let spec = if opt.argument.is_empty() {
                format!("'{}[{}]'", name.raw, desc)
            } else {
                format!("'{}[{} {}]'", name.raw, opt.argument, desc)
            };

            lines.push(format!("  options+=({})", spec));
        }

        lines
    }

    fn truncate_after_period(line: &str) -> String {
        line.split('.').next().unwrap_or("").to_string()
    }
}

pub struct BashGenerator;

impl BashGenerator {
    pub fn generate(cmd: &Command) -> String {
        Self::generate_with_compat(cmd, false)
    }

    pub fn generate_with_compat(cmd: &Command, bash_completion_compat: bool) -> String {
        let mut lines = vec![
            format!("_{}()", cmd.name),
            "{".to_string(),
            "  local cur prev opts".to_string(),
            "  COMPREPLY=()".to_string(),
            "  cur=\"${COMP_WORDS[COMP_CWORD]}\"".to_string(),
            "  prev=\"${COMP_WORDS[COMP_CWORD-1]}\"".to_string(),
            "".to_string(),
        ];

        let all_opts: Vec<String> = if bash_completion_compat {
            cmd.options
                .iter()
                .flat_map(|opt| {
                    let base_desc = FishGenerator::truncate_after_period(&opt.description);
                    let desc = base_desc
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
                                Some(name.raw.clone())
                            } else {
                                Some(format!("{}:{}", name.raw, desc))
                            }
                        })
                        .collect::<Vec<_>>()
                })
                .collect::<BTreeSet<_>>()
                .into_iter()
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
                                Some(name.raw.clone())
                            }
                        })
                        .collect::<Vec<_>>()
                })
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect()
        };

        lines.push(format!("  opts=\"{}\"", all_opts.join(" ")));
        lines.push("".to_string());
        lines.push("  COMPREPLY=($(compgen -W \"${opts}\" -- ${cur}))".to_string());

        if bash_completion_compat {
            lines.push("  if type __ltrim_colon_completions &>/dev/null; then".to_string());
            lines.push("    __ltrim_colon_completions \"$cur\"".to_string());
            lines.push("  fi".to_string());
        }

        lines.push("}".to_string());
        lines.push("".to_string());
        lines.push(format!(
            "complete -o bashdefault -o default -o nospace -F _{} {}",
            cmd.name, cmd.name
        ));

        lines.join("\n")
    }
}

pub struct ElvishGenerator;

impl ElvishGenerator {
    pub fn generate(cmd: &Command) -> String {
        let mut lines = Vec::new();
        lines.push("use builtin;".to_string());
        lines.push("use str;".to_string());
        lines.push("".to_string());
        lines.push(format!(
            "set edit:completion:arg-completer[{}] = {{|@words|",
            cmd.name
        ));
        lines.push("    fn spaces {|n|".to_string());
        lines.push("        builtin:repeat $n ' ' | str:join ''".to_string());
        lines.push("    }".to_string());
        lines.push("    fn cand {|text desc|".to_string());
        lines.push(
            "        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc"
                .to_string(),
        );
        lines.push("    }".to_string());
        lines.push(format!("    var command = '{}'", cmd.name));
        lines.push("    for word $words[1..-1] {".to_string());
        lines.push("        if (str:has-prefix $word '-') {".to_string());
        lines.push("            break".to_string());
        lines.push("        }".to_string());
        lines.push("        set command = $command';'$word".to_string());
        lines.push("    }".to_string());
        lines.push("    var completions = [".to_string());
        lines.push(format!("        &'{}'= {{", cmd.name));

        for opt in &cmd.options {
            let desc = FishGenerator::truncate_after_period(&opt.description);
            for name in &opt.names {
                if matches!(
                    name.opt_type,
                    OptNameType::SingleDashAlone | OptNameType::DoubleDashAlone
                ) {
                    continue;
                }
                lines.push(format!(
                    "            cand {} '{}'",
                    name.raw,
                    desc.replace("'", ""),
                ));
            }
        }

        lines.push("        }".to_string());
        lines.push("    ]".to_string());
        lines.push("    $completions[$command]".to_string());
        lines.push("}".to_string());

        lines.join("\n")
    }
}

pub struct NushellGenerator;

impl NushellGenerator {
    pub fn generate(cmd: &Command) -> String {
        let mut lines = Vec::new();
        lines.push("module completions {".to_string());
        lines.push("".to_string());

        lines.push(format!("  # Completions for {} options", cmd.name));
        lines.push(format!("  def \"nu-complete {} options\" [] {{", cmd.name));
        let all_opts: Vec<String> = cmd
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
                            Some(format!("\"{}\"", name.raw))
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();

        if all_opts.is_empty() {
            lines.push("    []".to_string());
        } else {
            let joined = all_opts.join(" ");
            lines.push(format!("    [ {} ]", joined));
        }
        lines.push("  }".to_string());
        lines.push("".to_string());

        lines.push(format!("  export extern {} [", cmd.name));

        for opt in &cmd.options {
            let desc = FishGenerator::truncate_after_period(&opt.description);
            let arg = if opt.argument.is_empty() {
                String::new()
            } else {
                format!(": string  # {}", opt.argument)
            };

            for name in &opt.names {
                if matches!(
                    name.opt_type,
                    OptNameType::SingleDashAlone | OptNameType::DoubleDashAlone
                ) {
                    continue;
                }

                let flag = format!("    {}{} # {}", name.raw, arg, desc);

                lines.push(flag);
            }
        }

        lines.push("  ]".to_string());
        lines.push("".to_string());
        lines.push("}".to_string());
        lines.push("".to_string());
        lines.push("export use completions *".to_string());

        lines.join("\n")
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
