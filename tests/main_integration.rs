use assert_cmd::cargo::cargo_bin_cmd;
use ecow::{EcoString, eco_vec};
use predicates::prelude::*;

/// Ensure running with no args shows clap error about missing input
#[test]
fn cli_errors_without_input_source() {
    let mut cmd = cargo_bin_cmd!("d2o");
    cmd.assert().failure().stderr(predicate::str::contains(
        "No input source specified. Use --command, --file, --subcommand, or --loadjson",
    ));
}

/// Smoke-test --help output
#[test]
fn cli_help_works() {
    let mut cmd = cargo_bin_cmd!("d2o");
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "d2o extracts CLI options from help text",
        ));
}

/// Use a tiny help text via --file and generate native output
#[test]
fn cli_file_native_output() {
    use std::io::Write;

    let mut tmp = tempfile::NamedTempFile::new().expect("create temp help");
    writeln!(
        tmp,
        "USAGE: mycmd [OPTIONS]\n\nOPTIONS:\n  -v, --verbose  be verbose"
    )
    .unwrap();
    let path = tmp.path().to_str().unwrap().to_string();

    let mut cmd = cargo_bin_cmd!("d2o");
    cmd.args(["--file", &path, "--format", "native"])
        .assert()
        .success()
        .stdout(predicate::str::contains("USAGE: mycmd [OPTIONS]"));
}

/// Verify --write caches output under ~/.d2o
#[test]
fn cli_write_caches_to_home_d2o() {
    use std::io::Write;

    let mut help_tmp = tempfile::NamedTempFile::new().expect("create temp help");
    writeln!(
        help_tmp,
        "USAGE: cachecmd [OPTIONS]\n\nOPTIONS:\n  -v, --verbose  be verbose"
    )
    .unwrap();
    let help_path = help_tmp.path().to_str().unwrap().to_string();

    let home_dir = tempfile::TempDir::new().expect("create temp home");

    let mut cmd = cargo_bin_cmd!("d2o");
    let assert = cmd
        .env("HOME", home_dir.path())
        .env("USERPROFILE", home_dir.path())
        .args(["--file", &help_path, "--format", "bash", "--write"])
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let stdout_trimmed = stdout.trim();

    // Path printed should exist and be under $HOME/.d2o
    let path = std::path::Path::new(stdout_trimmed);
    assert!(path.exists());
    assert!(
        path.starts_with(home_dir.path().join(".d2o")),
        "expected path under ~/.d2o, got {:?}",
        path
    );
}

/// Use the same help text but output JSON and ensure basic fields exist
#[test]
fn cli_file_json_output() {
    use std::io::Write;

    let mut tmp = tempfile::NamedTempFile::new().expect("create temp help");
    writeln!(
        tmp,
        "USAGE: mycmd [OPTIONS]\n\nOPTIONS:\n  -v, --verbose  be verbose"
    )
    .unwrap();
    let path = tmp.path().to_str().unwrap().to_string();

    let mut cmd = cargo_bin_cmd!("d2o");
    let assert = cmd
        .args(["--file", &path, "--format", "json"])
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let value: serde_json::Value = serde_json::from_str(&stdout).expect("valid json");

    let file_name = std::path::Path::new(&path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap();
    assert_eq!(value["name"], file_name);
    assert!(value["options"].is_array());
}

/// Ensure completions flag at least runs for bash
#[test]
fn cli_completions_bash() {
    let mut cmd = cargo_bin_cmd!("d2o");
    cmd.args(["--completions", "bash"])
        .assert()
        .success()
        .stdout(predicate::str::contains("_d2o"));
}

/// Test --list-subcommands path using a help snippet via --file
#[test]
fn cli_list_subcommands_from_file() {
    use std::io::Write;

    let mut tmp = tempfile::NamedTempFile::new().expect("create temp help");
    writeln!(
        tmp,
        "USAGE: mytool [COMMAND]\n\nSUBCOMMANDS:\n  run   Run things\n  build Build things",
    )
    .unwrap();
    let path = tmp.path().to_str().unwrap().to_string();

    let mut cmd = cargo_bin_cmd!("d2o");
    cmd.args(["--file", &path, "--list-subcommands"])
        .assert()
        .success()
        .stdout(predicate::str::contains("run").and(predicate::str::contains("build")));
}

/// Test debug/preprocess-only mode using --file
#[test]
fn cli_debug_preprocess_only() {
    use std::io::Write;

    let mut tmp = tempfile::NamedTempFile::new().expect("create temp help");
    writeln!(tmp, "OPTIONS:\n  -v, --verbose  be verbose",).unwrap();
    let path = tmp.path().to_str().unwrap().to_string();

    let mut cmd = cargo_bin_cmd!("d2o");
    cmd.args(["--file", &path, "--debug"])
        .assert()
        .success()
        .stdout(predicate::str::contains("-v, --verbose"));
}

/// Smoke-test --command echo with skip_man so it uses --help
#[test]
fn cli_command_echo_native() {
    let mut cmd = cargo_bin_cmd!("d2o");
    cmd.args(["--command", "echo", "--skip-man", "--format", "native"])
        .assert()
        .success();
}

/// Test --loadjson path end-to-end
#[test]
fn cli_loadjson_native_output() {
    use std::io::Write;

    let cmd_struct = d2o::Command {
        name: EcoString::from("jsoncmd"),
        description: EcoString::from("Json command"),
        usage: EcoString::from("jsoncmd [OPTIONS]"),
        options: eco_vec![d2o::types::Opt {
            names: eco_vec![d2o::types::OptName::new(
                EcoString::from("-v"),
                d2o::types::OptNameType::ShortType,
            )],
            argument: EcoString::new(),
            description: EcoString::from("Verbose"),
        }],
        subcommands: eco_vec![],
        version: EcoString::new(),
    };

    let json = serde_json::to_string(&cmd_struct).unwrap();
    let mut tmp = tempfile::NamedTempFile::new().expect("create json temp");
    write!(tmp, "{}", json).unwrap();
    let path = tmp.path().to_str().unwrap().to_string();

    let mut cmd = cargo_bin_cmd!("d2o");
    cmd.args(["--loadjson", &path, "--format", "native"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Name:  jsoncmd").and(predicate::str::contains("-v (")));
}
