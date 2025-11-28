use clap::{FromArgMatches, crate_name};
use clap_complete::generate;
use clap_complete::shells::{Bash, Elvish, Fish, PowerShell, Zsh};
use clap_complete_nushell::Nushell;
use d2o::{
    BashGenerator, Cache, Cli, Command, ElvishGenerator, FishGenerator, IoHandler, JsonGenerator,
    Layout, NushellGenerator, Postprocessor, Shell, SubcommandParser, ZshGenerator,
    command_with_version,
};
use ecow::EcoString;
use std::io;
use std::path::Path;
use std::time::Duration;
use tracing::debug;

#[cfg(not(any(target_arch = "arm", target_os = "freebsd", target_family = "wasm")))]
#[global_allocator]
static ALLOC: mimalloc_safe::MiMalloc = mimalloc_safe::MiMalloc;

fn init_tracing(cli: &Cli) {
    use tracing_subscriber::fmt;
    use tracing_subscriber::prelude::*;

    if let Some(level) = cli.verbosity.tracing_level() {
        tracing_subscriber::registry()
            .with(fmt::layer())
            .with(tracing_subscriber::filter::LevelFilter::from_level(level))
            .init();
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let raw_args = std::env::args_os();
    let expanded_args =
        argfile::expand_args_from(raw_args, argfile::parse_fromfile, argfile::PREFIX)?;

    // Parse using command_with_version() so -V shows long version
    let matches = command_with_version().get_matches_from(expanded_args);
    let cli = Cli::from_arg_matches(&matches)?;
    init_tracing(&cli);

    let mut command = command_with_version();
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

    // Handle cache operations
    if cli.cache_clear || cli.cache_stats {
        let ttl = Duration::from_secs(cli.cache_ttl * 3600);
        let cache = Cache::with_ttl(ttl)?;

        if cli.cache_clear {
            let count = cache.clear().await?;
            println!("Cleared {} cache entries", count);
        }

        if cli.cache_stats {
            let stats = cache.stats().await?;
            println!("{}", stats);
        }

        return Ok(());
    }

    let format = cli.effective_format().to_lowercase();

    // Handle preprocess only (debug mode)
    if cli.is_preprocess_only() {
        let content = get_input_content(&cli).await?;
        let pairs = Layout::preprocess_blockwise(&content);
        for (opt_part, desc) in pairs.iter() {
            println!("{}\n{}", opt_part, desc);
        }
        return Ok(());
    }

    // Handle list subcommands
    if cli.list_subcommands {
        let content = get_input_content(&cli).await?;
        let cmd = build_command(&cli, &content)?;
        for subcmd in cmd.subcommands.iter() {
            println!("{}", subcmd.name);
        }
        return Ok(());
    }

    // Normal processing with optional caching
    let cmd = if cli.loadjson.is_some() {
        load_command_from_json(&cli).await?
    } else {
        let content = get_input_content(&cli).await?;
        build_command_with_cache(&cli, &content).await?
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
        let path = write_output_to_cache(&cmd, &format, &output).await?;
        println!("{}", path.display());
    } else {
        println!("{}", output);
    }

    Ok(())
}

async fn get_input_content(cli: &Cli) -> anyhow::Result<EcoString> {
    let content = if let Some(json_file) = &cli.loadjson {
        IoHandler::read_file(json_file).await?
    } else if let Some(file) = &cli.file {
        IoHandler::read_file(file).await?
    } else if let Some(cmd_name) = &cli.command {
        if cli.skip_man || !IoHandler::is_man_available(cmd_name).await {
            IoHandler::get_command_help(cmd_name).await?
        } else {
            IoHandler::get_manpage(cmd_name).await?
        }
    } else if let Some(subcommand) = &cli.subcommand {
        let (cmd, subcmd) = subcommand.split_once('-').ok_or_else(|| {
            anyhow::anyhow!("Subcommand format should be command-subcommand (e.g., git-log)")
        })?;

        if cli.skip_man || !IoHandler::is_man_available(cmd).await {
            IoHandler::get_command_help(&format!("{} {}", cmd, subcmd)).await?
        } else {
            IoHandler::get_manpage(&format!("{}-{}", cmd, subcmd)).await?
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
        EcoString::from(cmd_name.as_str())
    } else if let Some(file) = &cli.file {
        EcoString::from(
            Path::new(file)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("command"),
        )
    } else if let Some(subcommand) = &cli.subcommand {
        EcoString::from(subcommand.as_str())
    } else {
        EcoString::from("command")
    };

    let mut cmd = Command::new(name.clone());
    cmd.options = Layout::parse_blockwise(content);
    cmd.usage = Layout::parse_usage(content);

    let subcommand_candidates = SubcommandParser::parse(content);
    if cli.depth > 0 && !subcommand_candidates.is_empty() {
        for subcmd in subcommand_candidates.iter() {
            let sub = Command {
                name: subcmd.cmd.clone(),
                description: subcmd.desc.clone(),
                usage: EcoString::new(),
                options: ecow::EcoVec::new(),
                subcommands: ecow::EcoVec::new(),
                version: EcoString::new(),
            };
            cmd.subcommands.push(sub);
        }
    }

    Ok(cmd)
}

/// Build a command with caching support.
async fn build_command_with_cache(cli: &Cli, content: &str) -> anyhow::Result<Command> {
    // Determine command name for cache key
    let name = cli
        .command
        .as_deref()
        .or(cli.subcommand.as_deref())
        .or_else(|| {
            cli.file
                .as_ref()
                .and_then(|f| Path::new(f).file_name()?.to_str())
        })
        .unwrap_or("command");

    // Determine source identifier for cache key
    let source = if cli.command.is_some() || cli.subcommand.is_some() {
        if cli.skip_man {
            Some("--help")
        } else {
            Some("man")
        }
    } else {
        cli.file.as_deref()
    };

    let content_hash = Cache::hash_content(content);

    // Try cache if enabled
    if cli.cache {
        let ttl = Duration::from_secs(cli.cache_ttl * 3600);
        if let Ok(cache) = Cache::with_ttl(ttl) {
            // Try to get from cache
            if let Some(cached_cmd) = cache.get(name, source, content_hash).await {
                debug!("Cache hit for command: {}", name);
                return Ok(cached_cmd);
            }

            // Parse and cache the result
            debug!("Cache miss for command: {}, parsing...", name);
            let cmd = build_command(cli, content)?;
            let cmd = Postprocessor::fix_command(cmd);

            // Store in cache (ignore errors, caching is best-effort)
            if let Err(e) = cache.set(name, source, content_hash, &cmd).await {
                debug!("Failed to cache command: {}", e);
            }

            return Ok(cmd);
        }
    }

    // Caching disabled or failed to initialize
    let cmd = build_command(cli, content)?;
    Ok(Postprocessor::fix_command(cmd))
}

async fn load_command_from_json(cli: &Cli) -> anyhow::Result<Command> {
    let json_file = cli
        .loadjson
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("No JSON file specified"))?;
    let content = IoHandler::read_file(json_file).await?;
    let mut cmd: Command = serde_json::from_str(&content)?;
    cmd = Postprocessor::fix_command(cmd);
    Ok(cmd)
}

fn format_native(cmd: &Command) -> EcoString {
    let mut output = Vec::new();

    output.push(format!("Name:  {}", cmd.name));
    output.push(format!("Desc:  {}", cmd.description));
    output.push(format!("Usage:\n{}", cmd.usage));

    for opt in cmd.options.iter() {
        output.push(format!(
            "  {} ({})",
            opt.names
                .iter()
                .map(|n| n.raw.to_string())
                .collect::<Vec<_>>()
                .join(", "),
            opt.argument
        ));
    }

    for subcmd in cmd.subcommands.iter() {
        output.push(format!("Subcommand: {}", subcmd.name));
    }

    EcoString::from(output.join("\n\n"))
}

async fn write_output_to_cache(
    cmd: &Command,
    format: &str,
    output: &str,
) -> anyhow::Result<std::path::PathBuf> {
    let home = std::env::home_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?;

    let mut dir = home;
    dir.push(".d2o");
    tokio::fs::create_dir_all(&dir).await?;

    let file_name = format!("{}.{}", cmd.name, format);
    let mut path = dir.clone();
    path.push(file_name);

    tokio::fs::write(&path, output).await?;

    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use d2o::cli::DEFAULT_CACHE_TTL_HOURS;
    use ecow::EcoVec;

    /// Helper to create a default Cli for testing
    fn test_cli() -> Cli {
        Cli {
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
            cache: false, // Disable cache in tests by default
            cache_ttl: DEFAULT_CACHE_TTL_HOURS,
            cache_clear: false,
            cache_stats: false,
            verbosity: Default::default(),
        }
    }

    #[tokio::test]
    async fn test_get_input_content_from_file() {
        use std::io::Write;

        let mut tmp = tempfile::NamedTempFile::new().expect("create temp file");
        writeln!(
            tmp,
            "USAGE: mycmd [OPTIONS]\n\nOPTIONS:\n  -v, --verbose  be verbose"
        )
        .unwrap();
        let path = tmp.path().to_str().unwrap().to_string();

        let cli = Cli {
            file: Some(path.clone()),
            ..test_cli()
        };

        let content = get_input_content(&cli).await.expect("get input from file");
        assert!(content.contains("USAGE: mycmd"));
    }

    #[tokio::test]
    async fn test_get_input_content_error_no_source() {
        let cli_no_input = test_cli();
        let err = get_input_content(&cli_no_input).await.unwrap_err();
        let msg = format!("{}", err);
        assert!(msg.contains("No input source specified"));
    }

    #[tokio::test]
    async fn test_load_command_from_json_roundtrip() {
        use std::io::Write;

        let cmd = Command {
            name: EcoString::from("jsoncmd"),
            description: EcoString::from("Json command"),
            usage: EcoString::from("jsoncmd [OPTIONS]"),
            options: {
                let mut v = EcoVec::new();
                v.push(d2o::types::Opt {
                    names: {
                        let mut names = EcoVec::new();
                        names.push(d2o::types::OptName::new(
                            EcoString::from("-v"),
                            d2o::types::OptNameType::ShortType,
                        ));
                        names
                    },
                    argument: EcoString::new(),
                    description: EcoString::from("Verbose"),
                });
                v
            },
            subcommands: EcoVec::new(),
            version: EcoString::new(),
        };

        let json = serde_json::to_string(&cmd).unwrap();

        let mut tmp = tempfile::NamedTempFile::new().expect("create json temp file");
        write!(tmp, "{}", json).unwrap();
        let path = tmp.path().to_str().unwrap().to_string();

        let cli = Cli {
            loadjson: Some(path),
            format: "json".to_string(),
            ..test_cli()
        };

        let loaded = load_command_from_json(&cli).await.expect("load from json");
        assert_eq!(loaded.name.as_str(), "jsoncmd");
        assert_eq!(loaded.options.len(), 1);
        assert_eq!(loaded.options[0].description.as_str(), "Verbose");
    }

    #[test]
    fn test_build_command_uses_command_name_and_parses_options() {
        let cli = Cli {
            command: Some("mycmd".to_string()),
            ..test_cli()
        };

        let help = "USAGE: mycmd [OPTIONS]\n\nOPTIONS:\n  -v, --verbose   be verbose";
        let cmd = build_command(&cli, help).expect("build command");

        assert_eq!(cmd.name.as_str(), "mycmd");
        assert!(cmd.usage.contains("mycmd"));
        assert_eq!(cmd.options.len(), 1);
        let opt = &cmd.options[0];
        let names: Vec<String> = opt.names.iter().map(|n| n.raw.to_string()).collect();
        assert!(names.contains(&"-v".to_string()));
        assert!(names.contains(&"--verbose".to_string()));
    }

    #[test]
    fn test_build_command_name_from_file_and_subcommands() {
        let cli = Cli {
            file: Some("/tmp/mycmd-help.txt".to_string()),
            depth: 1,
            ..test_cli()
        };

        let help =
            "USAGE: mycmd [COMMAND]\n\nSUBCOMMANDS:\n  run   Run things\n  build Build things";
        let cmd = build_command(&cli, help).expect("build command");

        assert_eq!(cmd.name.as_str(), "mycmd-help.txt");
        let names: Vec<String> = cmd.subcommands.iter().map(|s| s.name.to_string()).collect();
        assert!(names.contains(&"run".to_string()));
        assert!(names.contains(&"build".to_string()));
    }

    #[test]
    fn test_format_native_includes_fields() {
        let mut cmd = Command::new(EcoString::from("test"));
        cmd.description = EcoString::from("Test command");
        cmd.usage = EcoString::from("test [OPTIONS]");

        cmd.options.push(d2o::types::Opt {
            names: {
                let mut v = EcoVec::new();
                v.push(d2o::types::OptName::new(
                    EcoString::from("-v"),
                    d2o::types::OptNameType::ShortType,
                ));
                v.push(d2o::types::OptName::new(
                    EcoString::from("--verbose"),
                    d2o::types::OptNameType::LongType,
                ));
                v
            },
            argument: EcoString::from("FILE"),
            description: EcoString::from("Enable verbose mode"),
        });

        cmd.subcommands.push(Command {
            name: EcoString::from("sub"),
            description: EcoString::new(),
            usage: EcoString::new(),
            options: EcoVec::new(),
            subcommands: EcoVec::new(),
            version: EcoString::new(),
        });

        let out = format_native(&cmd);
        assert!(out.contains("Name:  test"));
        assert!(out.contains("Desc:  Test command"));
        assert!(out.contains("Usage:\ntest [OPTIONS]"));
        assert!(out.contains("-v, --verbose"));
        assert!(out.contains("Subcommand: sub"));
    }

    #[tokio::test]
    async fn test_build_command_with_cache_disabled() {
        let cli = Cli {
            command: Some("testcmd".to_string()),
            cache: false,
            ..test_cli()
        };

        let help = "USAGE: testcmd [OPTIONS]\n\nOPTIONS:\n  -v, --verbose  be verbose";
        let cmd = build_command_with_cache(&cli, help)
            .await
            .expect("build with cache disabled");

        assert_eq!(cmd.name.as_str(), "testcmd");
    }

    #[tokio::test]
    async fn test_build_command_with_cache_enabled() {
        let cli = Cli {
            command: Some("cachedcmd".to_string()),
            cache: true,
            cache_ttl: 1,
            ..test_cli()
        };

        let help = "USAGE: cachedcmd [OPTIONS]\n\nOPTIONS:\n  -v, --verbose  be verbose";

        // First call should parse and cache
        let cmd1 = build_command_with_cache(&cli, help)
            .await
            .expect("first build");
        assert_eq!(cmd1.name.as_str(), "cachedcmd");

        // Second call with same content should hit cache
        let cmd2 = build_command_with_cache(&cli, help)
            .await
            .expect("second build");
        assert_eq!(cmd2.name.as_str(), "cachedcmd");
        assert_eq!(cmd1.options.len(), cmd2.options.len());
    }
}
