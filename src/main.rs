use clap::{CommandFactory, Parser, crate_name};
use clap_complete::generate;
use clap_complete::shells::{Bash, Elvish, Fish, PowerShell, Zsh};
use clap_complete_nushell::Nushell;
use hcl::{
    BashGenerator, Cli, Command, ElvishGenerator, FishGenerator, IoHandler, JsonGenerator, Layout,
    NushellGenerator, Postprocessor, Shell, SubcommandParser, ZshGenerator,
};
use std::io;
use std::path::Path;

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let mut command = Cli::command();
    let name = crate_name!();
    let mut stdout = io::stdout();

    // Handle completions generation
    if let Some(shell) = cli.completions {
        match shell {
            Shell::Bash => generate(Bash, &mut command, name, &mut stdout),
            Shell::Fish => generate(Fish, &mut command, name, &mut stdout),
            Shell::Zsh => generate(Zsh, &mut command, name, &mut stdout),
            Shell::PowerShell => generate(PowerShell, &mut command, name, &mut stdout),
            Shell::Elvish => generate(Elvish, &mut command, name, &mut stdout),
            Shell::Nushell => generate(Nushell, &mut command, name, &mut stdout),
        }
        return Ok(());
    }

    let format = cli.effective_format().to_lowercase();

    // Handle preprocess only (debug mode)
    if cli.is_preprocess_only() {
        let content = get_input_content(&cli)?;
        let pairs = Layout::preprocess_blockwise(&content);
        for (opt_part, desc) in pairs {
            println!("{}\n{}", opt_part, desc);
        }
        return Ok(());
    }

    // Handle list subcommands
    if cli.list_subcommands {
        let content = get_input_content(&cli)?;
        let cmd = build_command(&cli, &content)?;
        for subcmd in cmd.subcommands {
            println!("{}", subcmd.name);
        }
        return Ok(());
    }

    // Normal processing
    let cmd = if cli.loadjson.is_some() {
        load_command_from_json(&cli)?
    } else {
        let content = get_input_content(&cli)?;
        let cmd = build_command(&cli, &content)?;
        Postprocessor::fix_command(cmd)
    };

    let output = match format.as_str() {
        "fish" => FishGenerator::generate(&cmd),
        "zsh" => ZshGenerator::generate(&cmd),
        "bash" => BashGenerator::generate_with_compat(&cmd, cli.bash_completion_compat),
        "elvish" => ElvishGenerator::generate(&cmd),
        "nushell" => NushellGenerator::generate(&cmd),
        "json" => JsonGenerator::generate(&cmd),
        "native" => format_native(&cmd),
        _ => anyhow::bail!("Unknown output option"),
    };
 
    if cli.write {
        let path = write_output_to_cache(&cmd, &format, &output)?;
        println!("{}", path.display());
    } else {
        println!("{}", output);
    }
 
    Ok(())
 }


fn get_input_content(cli: &Cli) -> anyhow::Result<String> {
    let content = if let Some(json_file) = &cli.loadjson {
        IoHandler::read_file(json_file)?
    } else if let Some(file) = &cli.file {
        IoHandler::read_file(file)?
    } else if let Some(cmd_name) = &cli.command {
        if cli.skip_man || !IoHandler::is_man_available(cmd_name) {
            IoHandler::get_command_help(cmd_name)?
        } else {
            IoHandler::get_manpage(cmd_name)?
        }
    } else if let Some(subcommand) = &cli.subcommand {
        let (cmd, subcmd) = subcommand.split_once('-').ok_or_else(|| {
            anyhow::anyhow!("Subcommand format should be command-subcommand (e.g., git-log)")
        })?;

        if cli.skip_man || !IoHandler::is_man_available(cmd) {
            IoHandler::get_command_help(&format!("{} {}", cmd, subcmd))?
        } else {
            IoHandler::get_manpage(&format!("{}-{}", cmd, subcmd))?
        }
    } else {
        return Err(anyhow::anyhow!(
            "No input source specified. Use --command, --file, --subcommand, or --loadjson"
        ));
    };

    Ok(Postprocessor::unicode_spaces_to_ascii(
        &Postprocessor::remove_bullets(&IoHandler::normalize_text(&content)),
    ))
}

fn build_command(cli: &Cli, content: &str) -> anyhow::Result<Command> {
    let name = if let Some(cmd_name) = &cli.command {
        cmd_name.clone()
    } else if let Some(file) = &cli.file {
        Path::new(file)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("command")
            .to_string()
    } else if let Some(subcommand) = &cli.subcommand {
        subcommand.clone()
    } else {
        "command".to_string()
    };

    let mut cmd = Command::new(name.clone());
    cmd.options = Layout::parse_blockwise(content);
    cmd.usage = Layout::parse_usage(content);

    let subcommand_candidates = SubcommandParser::parse(content);
    if cli.depth > 0 && !subcommand_candidates.is_empty() {
        for subcmd in subcommand_candidates {
            let sub = Command {
                name: subcmd.cmd,
                description: subcmd.desc,
                usage: String::new(),
                options: Vec::new(),
                subcommands: Vec::new(),
                version: String::new(),
            };
            cmd.subcommands.push(sub);
        }
    }

    Ok(cmd)
}

fn load_command_from_json(cli: &Cli) -> anyhow::Result<Command> {
    let json_file = cli
        .loadjson
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("No JSON file specified"))?;
    let content = IoHandler::read_file(json_file)?;
    let mut cmd: Command = serde_json::from_str(&content)?;
    cmd = Postprocessor::fix_command(cmd);
    Ok(cmd)
}

fn format_native(cmd: &Command) -> String {
    let mut output = Vec::new();
 
    output.push(format!("Name:  {}", cmd.name));
    output.push(format!("Desc:  {}", cmd.description));
    output.push(format!("Usage:\n{}", cmd.usage));
 
    for opt in &cmd.options {
        output.push(format!(
            "  {} ({})",
            opt.names
                .iter()
                .map(|n| n.raw.clone())
                .collect::<Vec<_>>()
                .join(", "),
            opt.argument
        ));
    }
 
    for subcmd in &cmd.subcommands {
        output.push(format!("Subcommand: {}", subcmd.name));
    }
 
    output.join("\n\n")
}
 
fn write_output_to_cache(
    cmd: &Command,
    format: &str,
    output: &str,
) -> anyhow::Result<std::path::PathBuf> {
    use std::fs;
    use std::path::PathBuf;
 
    let home = std::env::var("HOME")
        .map_err(|_| anyhow::anyhow!("HOME environment variable not set"))?;
 
    let mut dir = PathBuf::from(home);
    dir.push(".hcl");
    fs::create_dir_all(&dir)?;
 
    let file_name = format!("{}.{}", cmd.name, format);
    let mut path = dir.clone();
    path.push(file_name);
 
    fs::write(&path, output)?;
 
    Ok(path)
}
 
#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_get_input_content_from_file_and_error() {
        use std::io::Write;

        let mut tmp = tempfile::NamedTempFile::new().expect("create temp file");
        writeln!(tmp, "USAGE: mycmd [OPTIONS]\n\nOPTIONS:\n  -v, --verbose  be verbose").unwrap();
        let path = tmp.path().to_str().unwrap().to_string();

        let cli = Cli {
            command: None,
            file: Some(path.clone()),
            subcommand: None,
            loadjson: None,
            format: "native".to_string(),
            json: false,
            skip_man: false,
            list_subcommands: false,
            debug: false,
            depth: 4,
            completions: None,
            write: false,
            bash_completion_compat: false,
        };

        let content = get_input_content(&cli).expect("get input from file");
        assert!(content.contains("USAGE: mycmd"));

        let cli_no_input = Cli {
            command: None,
            file: None,
            subcommand: None,
            loadjson: None,
            format: "native".to_string(),
            json: false,
            skip_man: false,
            list_subcommands: false,
            debug: false,
            depth: 4,
            completions: None,
            write: false,
            bash_completion_compat: false,
        };

        let err = get_input_content(&cli_no_input).unwrap_err();
        let msg = format!("{}", err);
        assert!(msg.contains("No input source specified"));
    }

    #[test]
    fn test_load_command_from_json_roundtrip() {
        use std::io::Write;

        let cmd = Command {
            name: "jsoncmd".to_string(),
            description: "Json command".to_string(),
            usage: "jsoncmd [OPTIONS]".to_string(),
            options: vec![hcl::types::Opt {
                names: vec![hcl::types::OptName::new(
                    "-v".to_string(),
                    hcl::types::OptNameType::ShortType,
                )],
                argument: String::new(),
                description: "Verbose".to_string(),
            }],
            subcommands: vec![],
            version: String::new(),
        };

        let json = serde_json::to_string(&cmd).unwrap();

        let mut tmp = tempfile::NamedTempFile::new().expect("create json temp file");
        write!(tmp, "{}", json).unwrap();
        let path = tmp.path().to_str().unwrap().to_string();

        let cli = Cli {
            command: None,
            file: None,
            subcommand: None,
            loadjson: Some(path),
            format: "json".to_string(),
            json: false,
            skip_man: false,
            list_subcommands: false,
            debug: false,
            depth: 4,
            completions: None,
            write: false,
            bash_completion_compat: false,
        };

        let loaded = load_command_from_json(&cli).expect("load from json");
        assert_eq!(loaded.name, "jsoncmd");
        assert_eq!(loaded.options.len(), 1);
        assert_eq!(loaded.options[0].description, "Verbose");
    }

    #[test]
    fn test_build_command_uses_command_name_and_parses_options() {
        let cli = Cli {
            command: Some("mycmd".to_string()),
            file: None,
            subcommand: None,
            loadjson: None,
            format: "native".to_string(),
            json: false,
            skip_man: false,
            list_subcommands: false,
            debug: false,
            depth: 4,
            completions: None,
            write: false,
            bash_completion_compat: false,
        };

        let help = "USAGE: mycmd [OPTIONS]\n\nOPTIONS:\n  -v, --verbose   be verbose";
        let cmd = build_command(&cli, help).expect("build command");

        assert_eq!(cmd.name, "mycmd");
        assert!(cmd.usage.contains("mycmd"));
        assert_eq!(cmd.options.len(), 1);
        let opt = &cmd.options[0];
        let names: Vec<String> = opt.names.iter().map(|n| n.raw.clone()).collect();
        assert!(names.contains(&"-v".to_string()));
        assert!(names.contains(&"--verbose".to_string()));
    }

    #[test]
    fn test_build_command_name_from_file_and_subcommands() {
        let cli = Cli {
            command: None,
            file: Some("/tmp/mycmd-help.txt".to_string()),
            subcommand: None,
            loadjson: None,
            format: "native".to_string(),
            json: false,
            skip_man: false,
            list_subcommands: false,
            debug: false,
            depth: 1,
            completions: None,
            write: false,
            bash_completion_compat: false,
        };

        let help = "USAGE: mycmd [COMMAND]\n\nSUBCOMMANDS:\n  run   Run things\n  build Build things";
        let cmd = build_command(&cli, help).expect("build command");

        assert_eq!(cmd.name, "mycmd-help.txt".to_string());
        let names: Vec<String> = cmd.subcommands.iter().map(|s| s.name.clone()).collect();
        assert!(names.contains(&"run".to_string()));
        assert!(names.contains(&"build".to_string()));
    }

    #[test]
    fn test_format_native_includes_fields() {
        let mut cmd = Command::new("test".to_string());
        cmd.description = "Test command".to_string();
        cmd.usage = "test [OPTIONS]".to_string();

        cmd.options.push(hcl::types::Opt {
            names: vec![
                hcl::types::OptName::new("-v".to_string(), hcl::types::OptNameType::ShortType),
                hcl::types::OptName::new("--verbose".to_string(), hcl::types::OptNameType::LongType),
            ],
            argument: "FILE".to_string(),
            description: "Enable verbose mode".to_string(),
        });

        cmd.subcommands.push(Command {
            name: "sub".to_string(),
            description: String::new(),
            usage: String::new(),
            options: Vec::new(),
            subcommands: Vec::new(),
            version: String::new(),
        });

        let out = format_native(&cmd);
        assert!(out.contains("Name:  test"));
        assert!(out.contains("Desc:  Test command"));
        assert!(out.contains("Usage:\ntest [OPTIONS]"));
        assert!(out.contains("-v, --verbose"));
        assert!(out.contains("Subcommand: sub"));
    }
}

