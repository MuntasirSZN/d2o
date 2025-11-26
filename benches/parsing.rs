//! Benchmarks for parsing and generation using divan.
//!
//! Run with: cargo bench

use divan::AllocProfiler;
use divan::{Bencher, black_box};
use hcl::{
    BashGenerator, Command, ElvishGenerator, FishGenerator, JsonGenerator, Layout,
    NushellGenerator, Opt, OptName, OptNameType, Postprocessor, ZshGenerator,
};

#[global_allocator]
static ALLOC: AllocProfiler = AllocProfiler::system();

fn main() {
    divan::main();
}

// ============================================================================
// Sample data for benchmarks
// ============================================================================

fn sample_help_small() -> &'static str {
    r#"Usage: mycmd [OPTIONS]

Options:
  -h, --help       Print help information
  -V, --version    Print version information
  -v, --verbose    Enable verbose output
  -q, --quiet      Suppress output
  -f, --file FILE  Input file path
"#
}

fn sample_help_medium() -> String {
    let mut lines = vec!["Usage: mycmd [OPTIONS] [COMMAND]".to_string()];
    lines.push(String::new());
    lines.push("Options:".to_string());

    for i in 0..50 {
        lines.push(format!(
            "  -{}, --option-{}  Description for option {}",
            (b'a' + (i % 26)) as char,
            i,
            i
        ));
    }

    lines.push(String::new());
    lines.push("Commands:".to_string());
    for i in 0..20 {
        lines.push(format!("  subcmd{}    Subcommand {}", i, i));
    }

    lines.join("\n")
}

fn sample_help_large() -> String {
    let mut lines = vec!["Usage: largecmd [OPTIONS] [COMMAND]".to_string()];
    lines.push(String::new());
    lines.push("Options:".to_string());

    for i in 0..500 {
        lines.push(format!(
            "  --option-{:<4}  {}",
            i,
            "A".repeat(50 + (i % 100))
        ));
    }

    lines.join("\n")
}

fn sample_command_small() -> Command {
    Command {
        name: "mycmd".to_string(),
        description: "A sample command".to_string(),
        usage: "mycmd [OPTIONS]".to_string(),
        options: vec![
            Opt {
                names: vec![
                    OptName::new("-h".to_string(), OptNameType::ShortType),
                    OptName::new("--help".to_string(), OptNameType::LongType),
                ],
                argument: String::new(),
                description: "Print help".to_string(),
            },
            Opt {
                names: vec![
                    OptName::new("-v".to_string(), OptNameType::ShortType),
                    OptName::new("--verbose".to_string(), OptNameType::LongType),
                ],
                argument: String::new(),
                description: "Verbose output".to_string(),
            },
        ],
        subcommands: Vec::new(),
        version: "1.0.0".to_string(),
    }
}

fn sample_command_medium() -> Command {
    let options: Vec<Opt> = (0..50)
        .map(|i| Opt {
            names: vec![OptName::new(format!("--opt-{}", i), OptNameType::LongType)],
            argument: if i % 3 == 0 {
                "VALUE".to_string()
            } else {
                String::new()
            },
            description: format!("Option number {}", i),
        })
        .collect();

    let subcommands: Vec<Command> = (0..10)
        .map(|i| Command {
            name: format!("sub{}", i),
            description: format!("Subcommand {}", i),
            usage: String::new(),
            options: Vec::new(),
            subcommands: Vec::new(),
            version: String::new(),
        })
        .collect();

    Command {
        name: "mediumcmd".to_string(),
        description: "A medium-sized command".to_string(),
        usage: "mediumcmd [OPTIONS] [COMMAND]".to_string(),
        options,
        subcommands,
        version: "2.0.0".to_string(),
    }
}

fn sample_command_large() -> Command {
    let options: Vec<Opt> = (0..500)
        .map(|i| Opt {
            names: vec![
                OptName::new(
                    format!("-{}", (b'a' + (i % 26) as u8) as char),
                    OptNameType::ShortType,
                ),
                OptName::new(format!("--option-{}", i), OptNameType::LongType),
            ],
            argument: if i % 2 == 0 {
                "ARG".to_string()
            } else {
                String::new()
            },
            description: format!("This is the description for option number {}", i),
        })
        .collect();

    Command {
        name: "largecmd".to_string(),
        description: "A large command with many options".to_string(),
        usage: "largecmd [OPTIONS]".to_string(),
        options,
        subcommands: Vec::new(),
        version: "3.0.0".to_string(),
    }
}

// ============================================================================
// Parsing benchmarks
// ============================================================================

#[divan::bench]
fn parse_blockwise_small(bencher: Bencher) {
    let help = sample_help_small();
    bencher.bench_local(|| Layout::parse_blockwise(black_box(help)));
}

#[divan::bench]
fn parse_blockwise_medium(bencher: Bencher) {
    let help = sample_help_medium();
    bencher.bench_local(|| Layout::parse_blockwise(black_box(&help)));
}

#[divan::bench]
fn parse_blockwise_large(bencher: Bencher) {
    let help = sample_help_large();
    bencher.bench_local(|| Layout::parse_blockwise(black_box(&help)));
}

#[divan::bench]
fn parse_usage_small(bencher: Bencher) {
    let help = sample_help_small();
    bencher.bench_local(|| Layout::parse_usage(black_box(help)));
}

#[divan::bench]
fn parse_usage_medium(bencher: Bencher) {
    let help = sample_help_medium();
    bencher.bench_local(|| Layout::parse_usage(black_box(&help)));
}

#[divan::bench]
fn preprocess_blockwise_small(bencher: Bencher) {
    let help = sample_help_small();
    bencher.bench_local(|| Layout::preprocess_blockwise(black_box(help)));
}

#[divan::bench]
fn preprocess_blockwise_medium(bencher: Bencher) {
    let help = sample_help_medium();
    bencher.bench_local(|| Layout::preprocess_blockwise(black_box(&help)));
}

// ============================================================================
// Generator benchmarks
// ============================================================================

#[divan::bench]
fn generate_bash_small(bencher: Bencher) {
    let cmd = sample_command_small();
    bencher.bench_local(|| BashGenerator::generate(black_box(&cmd)));
}

#[divan::bench]
fn generate_bash_medium(bencher: Bencher) {
    let cmd = sample_command_medium();
    bencher.bench_local(|| BashGenerator::generate(black_box(&cmd)));
}

#[divan::bench]
fn generate_bash_large(bencher: Bencher) {
    let cmd = sample_command_large();
    bencher.bench_local(|| BashGenerator::generate(black_box(&cmd)));
}

#[divan::bench]
fn generate_zsh_small(bencher: Bencher) {
    let cmd = sample_command_small();
    bencher.bench_local(|| ZshGenerator::generate(black_box(&cmd)));
}

#[divan::bench]
fn generate_zsh_medium(bencher: Bencher) {
    let cmd = sample_command_medium();
    bencher.bench_local(|| ZshGenerator::generate(black_box(&cmd)));
}

#[divan::bench]
fn generate_zsh_large(bencher: Bencher) {
    let cmd = sample_command_large();
    bencher.bench_local(|| ZshGenerator::generate(black_box(&cmd)));
}

#[divan::bench]
fn generate_fish_small(bencher: Bencher) {
    let cmd = sample_command_small();
    bencher.bench_local(|| FishGenerator::generate(black_box(&cmd)));
}

#[divan::bench]
fn generate_fish_medium(bencher: Bencher) {
    let cmd = sample_command_medium();
    bencher.bench_local(|| FishGenerator::generate(black_box(&cmd)));
}

#[divan::bench]
fn generate_fish_large(bencher: Bencher) {
    let cmd = sample_command_large();
    bencher.bench_local(|| FishGenerator::generate(black_box(&cmd)));
}

#[divan::bench]
fn generate_elvish_medium(bencher: Bencher) {
    let cmd = sample_command_medium();
    bencher.bench_local(|| ElvishGenerator::generate(black_box(&cmd)));
}

#[divan::bench]
fn generate_nushell_medium(bencher: Bencher) {
    let cmd = sample_command_medium();
    bencher.bench_local(|| NushellGenerator::generate(black_box(&cmd)));
}

#[divan::bench]
fn generate_json_small(bencher: Bencher) {
    let cmd = sample_command_small();
    bencher.bench_local(|| JsonGenerator::generate(black_box(&cmd)));
}

#[divan::bench]
fn generate_json_medium(bencher: Bencher) {
    let cmd = sample_command_medium();
    bencher.bench_local(|| JsonGenerator::generate(black_box(&cmd)));
}

#[divan::bench]
fn generate_json_large(bencher: Bencher) {
    let cmd = sample_command_large();
    bencher.bench_local(|| JsonGenerator::generate(black_box(&cmd)));
}

// ============================================================================
// Postprocessor benchmarks
// ============================================================================

#[divan::bench]
fn postprocess_fix_command_small(bencher: Bencher) {
    let cmd = sample_command_small();
    bencher.bench_local(|| Postprocessor::fix_command(black_box(cmd.clone())));
}

#[divan::bench]
fn postprocess_fix_command_medium(bencher: Bencher) {
    let cmd = sample_command_medium();
    bencher.bench_local(|| Postprocessor::fix_command(black_box(cmd.clone())));
}

#[divan::bench]
fn postprocess_unicode_spaces(bencher: Bencher) {
    let text = "Hello\u{00A0}world\u{2003}with\u{2009}unicode\u{202F}spaces".repeat(100);
    bencher.bench_local(|| Postprocessor::unicode_spaces_to_ascii(black_box(&text)));
}

#[divan::bench]
fn postprocess_remove_bullets(bencher: Bencher) {
    let text = "• Item one\n• Item two\n• Item three\n".repeat(100);
    bencher.bench_local(|| Postprocessor::remove_bullets(black_box(&text)));
}

// ============================================================================
// JSON serialization benchmarks
// ============================================================================

#[divan::bench]
fn json_serialize_command_medium(bencher: Bencher) {
    let cmd = sample_command_medium();
    bencher.bench_local(|| serde_json::to_string(black_box(&cmd)).unwrap());
}

#[divan::bench]
fn json_deserialize_command_medium(bencher: Bencher) {
    let cmd = sample_command_medium();
    let json = serde_json::to_string(&cmd).unwrap();
    bencher.bench_local(|| {
        let _: Command = serde_json::from_str(black_box(&json)).unwrap();
    });
}

#[divan::bench]
fn json_roundtrip_command_medium(bencher: Bencher) {
    let cmd = sample_command_medium();
    bencher.bench_local(|| {
        let json = serde_json::to_string(black_box(&cmd)).unwrap();
        let _: Command = serde_json::from_str(&json).unwrap();
    });
}

// ============================================================================
// OptName benchmarks
// ============================================================================

#[divan::bench]
fn optname_from_text_short(bencher: Bencher) {
    bencher.bench_local(|| OptName::from_text(black_box("-v")));
}

#[divan::bench]
fn optname_from_text_long(bencher: Bencher) {
    bencher.bench_local(|| OptName::from_text(black_box("--verbose")));
}

#[divan::bench]
fn optname_from_text_batch(bencher: Bencher) {
    let names = [
        "-a",
        "-b",
        "-c",
        "--help",
        "--version",
        "--verbose",
        "--quiet",
    ];
    bencher.bench_local(|| {
        for name in &names {
            let _ = OptName::from_text(black_box(name));
        }
    });
}
