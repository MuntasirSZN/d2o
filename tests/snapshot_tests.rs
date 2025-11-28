use clap::Parser as ClapParser;
use d2o::types::OptNameType;
use d2o::{
    BashGenerator, Cli, Command, ElvishGenerator, FishGenerator, NushellGenerator, Opt, OptName,
    Parser as D2oParser, ZshGenerator,
};
use ecow::{EcoString, eco_vec};

#[test]
fn test_parse_ls_help_snapshot() {
    let ls_help = r#"
OPTIONS:
  -a, --all                 do not ignore entries starting with .
  -A, --almost-all          do not list implied . and ..
  -b, --escape              print C-style escapes for nongraphic characters
"#;

    let opts = D2oParser::parse_line(ls_help);
    insta::assert_yaml_snapshot!(opts.len());
}

#[test]
fn test_zsh_generator_with_descriptions_snapshot() {
    let cmd = Command {
        name: EcoString::from("test"),
        description: EcoString::from("Test command"),
        usage: EcoString::from("test [OPTIONS]"),
        options: eco_vec![Opt {
            names: eco_vec![
                OptName::new(EcoString::from("-v"), OptNameType::ShortType),
                OptName::new(EcoString::from("--verbose"), OptNameType::LongType),
            ],
            argument: EcoString::new(),
            description: EcoString::from("Enable verbose mode"),
        }],
        subcommands: eco_vec![],
        version: EcoString::new(),
    };

    let output = ZshGenerator::generate(&cmd);
    insta::assert_snapshot!(output);
}

#[test]
fn test_parse_docker_help_snapshot() {
    let docker_help = r#"
Options:
  -d, --detach              Run container in background
  --name string             Assign a name to the container
  -p, --publish list        Publish a container's port(s) to the host
"#;

    let opts = D2oParser::parse_line(docker_help);
    insta::assert_yaml_snapshot!(opts.len());
}

#[test]
fn test_elvish_generator_snapshot() {
    let cmd = Command {
        name: EcoString::from("test"),
        description: EcoString::from("Test command"),
        usage: EcoString::from("test [OPTIONS]"),
        options: eco_vec![Opt {
            names: eco_vec![
                OptName::new(EcoString::from("-v"), OptNameType::ShortType),
                OptName::new(EcoString::from("--verbose"), OptNameType::LongType),
            ],
            argument: EcoString::new(),
            description: EcoString::from("Enable verbose mode"),
        }],
        subcommands: eco_vec![],
        version: EcoString::new(),
    };

    let output = ElvishGenerator::generate(&cmd);
    insta::assert_snapshot!(output);
}

#[test]
fn test_nushell_generator_snapshot() {
    let cmd = Command {
        name: EcoString::from("test"),
        description: EcoString::from("Test command"),
        usage: EcoString::from("test [OPTIONS]"),
        options: eco_vec![Opt {
            names: eco_vec![
                OptName::new(EcoString::from("-v"), OptNameType::ShortType),
                OptName::new(EcoString::from("--verbose"), OptNameType::LongType),
            ],
            argument: EcoString::new(),
            description: EcoString::from("Enable verbose mode"),
        }],
        subcommands: eco_vec![],
        version: EcoString::new(),
    };

    let output = NushellGenerator::generate(&cmd);
    insta::assert_snapshot!(output);
}

#[test]
fn test_cli_short_f_and_conflicts() {
    // -f should work as shorthand for --file
    let cli = Cli::try_parse_from(["d2o", "-f", "file.txt", "--format", "json"]).unwrap();
    assert_eq!(cli.file.as_deref(), Some("file.txt"));

    // Conflicting flags should error
    let res = Cli::try_parse_from(["d2o", "--command", "ls", "--file", "file.txt"]);
    assert!(res.is_err());
}

#[test]
fn test_cli_effective_format_and_helpers() {
    let cli = Cli::try_parse_from(["d2o", "--command", "ls", "--format", "bash"]).unwrap();

    // json flag off, effective_format should be underlying format
    assert_eq!(cli.effective_format(), "bash");
    assert_eq!(cli.get_input(), Some("ls"));
    assert!(!cli.is_preprocess_only());

    let cli_json =
        Cli::try_parse_from(["d2o", "--command", "ls", "--format", "bash", "--json"]).unwrap();

    // json flag forces json format
    assert_eq!(cli_json.effective_format(), "json");
}

#[test]
fn test_bash_generator_snapshot() {
    let cmd = Command {
        name: EcoString::from("test"),
        description: EcoString::from("Test command"),
        usage: EcoString::from("test [OPTIONS]"),
        options: eco_vec![Opt {
            names: eco_vec![
                OptName::new(EcoString::from("-v"), OptNameType::ShortType),
                OptName::new(EcoString::from("--verbose"), OptNameType::LongType),
            ],
            argument: EcoString::new(),
            description: EcoString::from("Enable verbose mode"),
        }],
        subcommands: eco_vec![],
        version: EcoString::new(),
    };

    let output = BashGenerator::generate(&cmd);
    insta::assert_snapshot!(output);
}

#[test]
fn test_bash_generator_compat_snapshot() {
    let cmd = Command {
        name: EcoString::from("test"),
        description: EcoString::from("Test command"),
        usage: EcoString::from("test [OPTIONS]"),
        options: eco_vec![Opt {
            names: eco_vec![
                OptName::new(EcoString::from("-v"), OptNameType::ShortType),
                OptName::new(EcoString::from("--verbose"), OptNameType::LongType),
            ],
            argument: EcoString::new(),
            description: EcoString::from("Enable verbose mode"),
        }],
        subcommands: eco_vec![],
        version: EcoString::new(),
    };

    let output = BashGenerator::generate_with_compat(&cmd, true);
    insta::assert_snapshot!(output);
}

#[test]
fn test_fish_generator_snapshot() {
    let cmd = Command {
        name: EcoString::from("test"),
        description: EcoString::from("Test command"),
        usage: EcoString::from("test [OPTIONS]"),
        options: eco_vec![Opt {
            names: eco_vec![
                OptName::new(EcoString::from("-v"), OptNameType::ShortType),
                OptName::new(EcoString::from("--verbose"), OptNameType::LongType),
            ],
            argument: EcoString::from("FILE"),
            description: EcoString::from("Enable verbose mode using a file"),
        }],
        subcommands: eco_vec![],
        version: EcoString::new(),
    };

    let output = FishGenerator::generate(&cmd);
    insta::assert_snapshot!(output);
}
