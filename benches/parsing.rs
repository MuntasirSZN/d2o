//! Benchmarks for parsing and generation using divan.
//!
//! Run with: cargo bench

use divan::AllocProfiler;
use divan::{Bencher, black_box};
use ecow::{EcoString, EcoVec, eco_vec};
use hcl::{
    BashGenerator, Command, ElvishGenerator, FishGenerator, JsonGenerator, Layout,
    NushellGenerator, Opt, OptName, OptNameType, Postprocessor, ZshGenerator,
};
use mimalloc_safe::MiMalloc;

#[cfg(not(any(target_arch = "arm", target_os = "freebsd")))]
#[global_allocator]
static ALLOC: AllocProfiler<MiMalloc> = AllocProfiler::new(MiMalloc);

#[cfg(any(target_arch = "arm", target_os = "freebsd"))]
static ALLOC: AllocProfiler = AllocProfilder::system();

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
        name: EcoString::from("mycmd"),
        description: EcoString::from("A sample command"),
        usage: EcoString::from("mycmd [OPTIONS]"),
        options: eco_vec![
            Opt {
                names: eco_vec![
                    OptName::new(EcoString::from("-h"), OptNameType::ShortType),
                    OptName::new(EcoString::from("--help"), OptNameType::LongType),
                ],
                argument: EcoString::new(),
                description: EcoString::from("Print help"),
            },
            Opt {
                names: eco_vec![
                    OptName::new(EcoString::from("-v"), OptNameType::ShortType),
                    OptName::new(EcoString::from("--verbose"), OptNameType::LongType),
                ],
                argument: EcoString::new(),
                description: EcoString::from("Verbose output"),
            },
        ],
        subcommands: eco_vec![],
        version: EcoString::from("1.0.0"),
    }
}

fn sample_command_medium() -> Command {
    let options: EcoVec<Opt> = (0..50)
        .map(|i| Opt {
            names: eco_vec![OptName::new(
                EcoString::from(format!("--opt-{}", i)),
                OptNameType::LongType
            )],
            argument: if i % 3 == 0 {
                EcoString::from("VALUE")
            } else {
                EcoString::new()
            },
            description: EcoString::from(format!("Option number {}", i)),
        })
        .collect();

    let subcommands: EcoVec<Command> = (0..10)
        .map(|i| Command {
            name: EcoString::from(format!("sub{}", i)),
            description: EcoString::from(format!("Subcommand {}", i)),
            usage: EcoString::new(),
            options: eco_vec![],
            subcommands: eco_vec![],
            version: EcoString::new(),
        })
        .collect();

    Command {
        name: EcoString::from("mediumcmd"),
        description: EcoString::from("A medium-sized command"),
        usage: EcoString::from("mediumcmd [OPTIONS] [COMMAND]"),
        options,
        subcommands,
        version: EcoString::from("2.0.0"),
    }
}

fn sample_command_large() -> Command {
    let options: EcoVec<Opt> = (0..500)
        .map(|i| Opt {
            names: eco_vec![
                OptName::new(
                    EcoString::from(format!("-{}", (b'a' + (i % 26) as u8) as char)),
                    OptNameType::ShortType,
                ),
                OptName::new(
                    EcoString::from(format!("--option-{}", i)),
                    OptNameType::LongType
                ),
            ],
            argument: if i % 2 == 0 {
                EcoString::from("ARG")
            } else {
                EcoString::new()
            },
            description: EcoString::from(format!(
                "This is the description for option number {}",
                i
            )),
        })
        .collect();

    Command {
        name: EcoString::from("largecmd"),
        description: EcoString::from("A large command with many options"),
        usage: EcoString::from("largecmd [OPTIONS]"),
        options,
        subcommands: eco_vec![],
        version: EcoString::from("3.0.0"),
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

// ============================================================================
// Massive file benchmarks (100+ MB stress tests)
// ============================================================================

fn sample_help_massive() -> String {
    // Generates ~1MB of help text (scaled down from 100MB for practical benchmarking)
    // Each option line is ~80 bytes, 10000 options = ~800KB
    let mut lines = vec!["Usage: massivecmd [OPTIONS] [COMMAND]".to_string()];
    lines.push(String::new());
    lines.push("Options:".to_string());

    for i in 0..10000 {
        lines.push(format!(
            "  -{}, --option-{:<6}  {}",
            (b'a' + (i % 26) as u8) as char,
            i,
            "Description text that provides detailed information about this option and its usage patterns in various scenarios."
        ));
    }

    lines.push(String::new());
    lines.push("Commands:".to_string());
    for i in 0..500 {
        lines.push(format!(
            "  subcmd{:<6}    Subcommand {} with a detailed description of what it does",
            i, i
        ));
    }

    lines.join("\n")
}

fn sample_help_10mb() -> String {
    // Generates ~10MB of help text
    let mut lines = vec!["Usage: hugecmd [OPTIONS] [COMMAND]".to_string()];
    lines.push(String::new());
    lines.push("Options:".to_string());

    for i in 0..100000 {
        lines.push(format!(
            "  --option-{:<8}  {}",
            i,
            "A".repeat(50 + (i % 50))
        ));
    }

    lines.join("\n")
}

fn sample_command_massive() -> Command {
    let options: EcoVec<Opt> = (0..5000)
        .map(|i| Opt {
            names: eco_vec![
                OptName::new(
                    EcoString::from(format!("-{}", (b'a' + (i % 26) as u8) as char)),
                    OptNameType::ShortType,
                ),
                OptName::new(
                    EcoString::from(format!("--option-{}", i)),
                    OptNameType::LongType
                ),
            ],
            argument: if i % 2 == 0 {
                EcoString::from("ARG")
            } else {
                EcoString::new()
            },
            description: EcoString::from(format!(
                "This is the description for option number {} with additional context",
                i
            )),
        })
        .collect();

    Command {
        name: EcoString::from("massivecmd"),
        description: EcoString::from("A massive command with thousands of options"),
        usage: EcoString::from("massivecmd [OPTIONS]"),
        options,
        subcommands: eco_vec![],
        version: EcoString::from("1.0.0"),
    }
}

#[divan::bench]
fn parse_blockwise_massive(bencher: Bencher) {
    let help = sample_help_massive();
    bencher.bench_local(|| Layout::parse_blockwise(black_box(&help)));
}

#[divan::bench]
fn parse_blockwise_10mb(bencher: Bencher) {
    let help = sample_help_10mb();
    bencher.bench_local(|| Layout::parse_blockwise(black_box(&help)));
}

#[divan::bench]
fn preprocess_blockwise_massive(bencher: Bencher) {
    let help = sample_help_massive();
    bencher.bench_local(|| Layout::preprocess_blockwise(black_box(&help)));
}

#[divan::bench]
fn generate_bash_massive(bencher: Bencher) {
    let cmd = sample_command_massive();
    bencher.bench_local(|| BashGenerator::generate(black_box(&cmd)));
}

#[divan::bench]
fn generate_zsh_massive(bencher: Bencher) {
    let cmd = sample_command_massive();
    bencher.bench_local(|| ZshGenerator::generate(black_box(&cmd)));
}

#[divan::bench]
fn generate_fish_massive(bencher: Bencher) {
    let cmd = sample_command_massive();
    bencher.bench_local(|| FishGenerator::generate(black_box(&cmd)));
}

#[divan::bench]
fn generate_json_massive(bencher: Bencher) {
    let cmd = sample_command_massive();
    bencher.bench_local(|| JsonGenerator::generate(black_box(&cmd)));
}

#[divan::bench]
fn postprocess_fix_command_massive(bencher: Bencher) {
    let cmd = sample_command_massive();
    bencher.bench_local(|| Postprocessor::fix_command(black_box(cmd.clone())));
}

#[divan::bench]
fn postprocess_unicode_spaces_massive(bencher: Bencher) {
    let text = "Hello\u{00A0}world\u{2003}with\u{2009}unicode\u{202F}spaces".repeat(10000);
    bencher.bench_local(|| Postprocessor::unicode_spaces_to_ascii(black_box(&text)));
}

#[divan::bench]
fn postprocess_remove_bullets_massive(bencher: Bencher) {
    let text = "• Item one\n• Item two\n• Item three\n".repeat(10000);
    bencher.bench_local(|| Postprocessor::remove_bullets(black_box(&text)));
}
