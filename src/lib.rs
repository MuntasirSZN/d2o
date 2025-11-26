pub mod cache;
pub mod cli;
pub mod generators;
pub mod io_handler;
pub mod json_gen;
pub mod layout;
pub mod parser;
pub mod postprocessor;
pub mod subcommand_parser;
pub mod types;

pub use cache::{Cache, CacheEntry, CacheStats, DEFAULT_TTL_SECS};
pub use cli::{Cli, Shell};
pub use generators::{
    BashGenerator, ElvishGenerator, FishGenerator, NushellGenerator, ZshGenerator,
};
pub use io_handler::IoHandler;
pub use json_gen::JsonGenerator;
pub use layout::Layout;
pub use parser::Parser;
pub use postprocessor::Postprocessor;
pub use subcommand_parser::SubcommandParser;
pub use types::*;

use shadow_rs::shadow;
shadow!(build);

/// Get the clap Command with long_version set from shadow-rs build info.
pub fn command_with_version() -> clap::Command {
    use clap::CommandFactory;
    Cli::command().long_version(build::CLAP_LONG_VERSION)
}

#[cfg(test)]
mod h2o_compat_tests {
    use super::*;

    #[test]
    fn h2o_json_schema_compatible() {
        let data = std::fs::read_to_string("tests/golden/h2o.json").expect("read h2o.json");
        let cmd: Command = serde_json::from_str(&data).expect("parse h2o Command JSON");

        assert_eq!(cmd.name, "h2o");
        assert_eq!(cmd.description, "h2o");
        assert!(cmd.usage.contains("--command"));
        assert!(!cmd.options.is_empty());
    }

    #[test]
    fn stack_json_schema_compatible() {
        let data = std::fs::read_to_string("tests/golden/stack.json").expect("read stack.json");
        let cmd: Command = serde_json::from_str(&data).expect("parse stack Command JSON");

        assert_eq!(cmd.name, "stack");
        assert_eq!(cmd.description, "stack");
        assert!(cmd.usage.contains("stack [--help]"));
        assert!(!cmd.options.is_empty());
        assert!(!cmd.subcommands.is_empty());
    }

    #[test]
    fn h2o_json_roundtrip_preserves_structure() {
        use serde_json::Value;

        let data = std::fs::read_to_string("tests/golden/h2o.json").expect("read h2o.json");
        let original: Value = serde_json::from_str(&data).expect("parse original h2o JSON");
        let cmd: Command = serde_json::from_str(&data).expect("parse h2o Command JSON");

        let generated_str = JsonGenerator::generate(&cmd);
        let generated: Value =
            serde_json::from_str(&generated_str).expect("parse generated h2o JSON");

        assert_eq!(original["name"], generated["name"]);
        assert_eq!(original["description"], generated["description"]);
        assert_eq!(original["usage"], generated["usage"]);
        assert_eq!(original["version"], generated["version"]);

        let orig_opts = original["options"].as_array().expect("options array");
        let gen_opts = generated["options"].as_array().expect("options array");
        assert_eq!(orig_opts.len(), gen_opts.len());

        for (o, g) in orig_opts.iter().zip(gen_opts.iter()) {
            assert_eq!(o["names"], g["names"], "option names differ");
            assert_eq!(o["argument"], g["argument"], "option argument differs");
            assert_eq!(
                o["description"], g["description"],
                "option description differs"
            );
        }
    }

    #[test]
    fn stack_json_roundtrip_preserves_top_level_invariants() {
        use serde_json::Value;

        let data = std::fs::read_to_string("tests/golden/stack.json").expect("read stack.json");
        let original: Value = serde_json::from_str(&data).expect("parse original stack JSON");
        let cmd: Command = serde_json::from_str(&data).expect("parse stack Command JSON");

        let generated_str = JsonGenerator::generate(&cmd);
        let generated: Value =
            serde_json::from_str(&generated_str).expect("parse generated stack JSON");

        assert_eq!(original["name"], generated["name"]);
        assert_eq!(original["description"], generated["description"]);
        assert_eq!(original["usage"], generated["usage"]);

        let orig_opts = original["options"].as_array().expect("options array");
        let gen_opts = generated["options"].as_array().expect("options array");
        assert_eq!(orig_opts.len(), gen_opts.len());

        for (o, g) in orig_opts.iter().zip(gen_opts.iter()) {
            assert_eq!(o["names"], g["names"], "option names differ");
            assert_eq!(o["argument"], g["argument"], "option argument differs");
            assert_eq!(
                o["description"], g["description"],
                "option description differs"
            );
        }

        let orig_subs = original["subcommands"]
            .as_array()
            .expect("subcommands array");
        let gen_subs = generated["subcommands"]
            .as_array()
            .expect("subcommands array");
        assert_eq!(orig_subs.len(), gen_subs.len());

        for (o, g) in orig_subs.iter().zip(gen_subs.iter()) {
            assert_eq!(o["name"], g["name"], "subcommand name differs");
            assert_eq!(
                o["description"], g["description"],
                "subcommand description differs"
            );
        }
    }
}
