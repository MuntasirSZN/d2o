use clap::{CommandFactory, Parser};
use clap_complete::generate;
use clap_complete::shells::{Bash, Elvish, Fish, PowerShell, Zsh};
use clap_complete_nushell::Nushell;
use hcl::{
    BashGenerator, Cli, Command, FishGenerator, IoHandler, JsonGenerator, Layout, Postprocessor,
    Shell, SubcommandParser, ZshGenerator,
};
use std::io;
use std::path::Path;

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Handle completions generation
    if let Some(shell) = cli.completions {
        match shell {
            Shell::Bash => generate(Bash, &mut Cli::command(), "hcl", &mut io::stdout()),
            Shell::Fish => generate(Fish, &mut Cli::command(), "hcl", &mut io::stdout()),
            Shell::Zsh => generate(Zsh, &mut Cli::command(), "hcl", &mut io::stdout()),
            Shell::PowerShell => {
                generate(PowerShell, &mut Cli::command(), "hcl", &mut io::stdout())
            }
            Shell::Elvish => generate(Elvish, &mut Cli::command(), "hcl", &mut io::stdout()),
            Shell::Nushell => generate(Nushell, &mut Cli::command(), "hcl", &mut io::stdout()),
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
    let content = get_input_content(&cli)?;
    let mut cmd = build_command(&cli, &content)?;
    cmd = Postprocessor::fix_command(cmd);

    let output = match format.as_str() {
        "fish" => FishGenerator::generate(&cmd),
        "zsh" => ZshGenerator::generate(&cmd),
        "bash" => BashGenerator::generate(&cmd),
        "json" => JsonGenerator::generate(&cmd),
        "native" => format_native(&cmd),
        _ => anyhow::bail!("Unknown output option"),
    };

    println!("{}", output);
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
        return anyhow::bail!(
            "No input source specified. Use --command, --file, --subcommand, or --loadjson"
        );
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
