use clap::CommandFactory;
use clap_complete::aot::generate_to;
use clap_complete::shells::{Bash, Elvish, Fish, PowerShell, Zsh};
use clap_complete_nushell::Nushell;
use std::fs;
use std::path::PathBuf;

include!("src/cli.rs");

fn main() {
    // Generate build-time metadata (including CLAP_LONG_VERSION)
    shadow_rs::ShadowBuilder::builder()
        .build_pattern(shadow_rs::BuildPattern::RealTime)
        .build()
        .expect("shadow-rs build failed");

    let out_dir = PathBuf::new().join("out");
    let mut cmd = Cli::command();

    let completions_dir = out_dir.join("completions");
    fs::create_dir_all(&completions_dir).unwrap();

    // Generate all standard shells
    generate_to(Bash, &mut cmd, "hcl", &completions_dir)
        .expect("Failed to generate bash completion");
    generate_to(Fish, &mut cmd, "hcl", &completions_dir)
        .expect("Failed to generate fish completion");
    generate_to(Zsh, &mut cmd, "hcl", &completions_dir).expect("Failed to generate zsh completion");
    generate_to(PowerShell, &mut cmd, "hcl", &completions_dir)
        .expect("Failed to generate powershell completion");
    generate_to(Elvish, &mut cmd, "hcl", &completions_dir)
        .expect("Failed to generate elvish completion");

    // Generate nushell
    generate_to(Nushell, &mut cmd, "hcl", &completions_dir)
        .expect("Failed to generate nushell completion");

    let man_dir = out_dir.join("man");
    fs::create_dir_all(&man_dir).unwrap();

    clap_mangen::generate_to(cmd, &man_dir).expect("Failed to generate manpage");

    println!("cargo:rerun-if-changed=src/cli.rs");
}
