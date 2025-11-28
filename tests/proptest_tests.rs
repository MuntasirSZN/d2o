//! Property-based tests using proptest for the d2o crate.
//!
//! These tests verify invariants that should hold for any valid input,
//! helping catch edge cases that unit tests might miss.

use d2o::{
    BashGenerator, Command, ElvishGenerator, FishGenerator, JsonGenerator, Layout,
    NushellGenerator, Opt, OptName, OptNameType, Postprocessor, ZshGenerator,
};
use ecow::{EcoString, EcoVec, eco_vec};
use proptest::prelude::*;

// ============================================================================
// Strategies for generating test data
// ============================================================================

/// Generate a valid short option name (e.g., "-a", "-Z", "-1")
fn short_opt_name() -> impl Strategy<Value = String> {
    prop::sample::select(vec![
        'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r',
        's', 't', 'u', 'v', 'w', 'x', 'y', 'z', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J',
        'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z', '0', '1',
        '2', '3', '4', '5', '6', '7', '8', '9',
    ])
    .prop_map(|c| format!("-{}", c))
}

/// Generate a valid long option name (e.g., "--help", "--verbose")
fn long_opt_name() -> impl Strategy<Value = String> {
    "[a-z][a-z0-9-]{0,20}".prop_map(|s| format!("--{}", s))
}

/// Generate a valid option name (short or long)
fn opt_name_strategy() -> impl Strategy<Value = OptName> {
    prop_oneof![
        short_opt_name().prop_map(|s| OptName::new(EcoString::from(s), OptNameType::ShortType)),
        long_opt_name().prop_map(|s| OptName::new(EcoString::from(s), OptNameType::LongType)),
    ]
}

/// Generate an argument placeholder (e.g., "FILE", "NUM", "<path>")
fn arg_strategy() -> impl Strategy<Value = EcoString> {
    prop_oneof![
        Just(EcoString::new()),
        Just(EcoString::from("FILE")),
        Just(EcoString::from("NUM")),
        Just(EcoString::from("PATH")),
        Just(EcoString::from("<value>")),
        "[a-zA-Z_]{1,10}".prop_map(|s| EcoString::from(s.to_uppercase())),
    ]
}

/// Generate a description string
fn description_strategy() -> impl Strategy<Value = EcoString> {
    prop_oneof![
        Just(EcoString::new()),
        "[a-zA-Z ]{0,50}".prop_map(EcoString::from),
        "Enable [a-z]+ mode".prop_map(EcoString::from),
        "Set the [a-z]+ value".prop_map(EcoString::from),
    ]
}

/// Generate a single Opt
fn opt_strategy() -> impl Strategy<Value = Opt> {
    (
        prop::collection::vec(opt_name_strategy(), 1..=3),
        arg_strategy(),
        description_strategy(),
    )
        .prop_map(|(names, argument, description)| Opt {
            names: names.into_iter().collect::<EcoVec<_>>(),
            argument,
            description,
        })
}

/// Generate a Command with options
fn command_strategy() -> impl Strategy<Value = Command> {
    (
        "[a-z][a-z0-9-]{0,15}",                       // name
        "[A-Za-z ]{0,30}",                            // description
        prop::collection::vec(opt_strategy(), 0..10), // options
    )
        .prop_map(|(name, description, options)| Command {
            name: EcoString::from(name),
            description: EcoString::from(description),
            usage: EcoString::new(),
            options: options.into_iter().collect::<EcoVec<_>>(),
            subcommands: eco_vec![],
            version: EcoString::new(),
        })
}

/// Generate help text in a standard format
fn help_text_strategy() -> impl Strategy<Value = String> {
    prop::collection::vec(
        (short_opt_name(), long_opt_name(), description_strategy()),
        1..=10,
    )
    .prop_map(|opts| {
        let mut lines = vec!["Usage: cmd [OPTIONS]".to_string(), String::new()];
        lines.push("Options:".to_string());
        for (short, long, desc) in opts {
            lines.push(format!("  {}, {}    {}", short, long, desc));
        }
        lines.join("\n")
    })
}

// ============================================================================
// Property tests for OptName
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]

    #[test]
    fn optname_from_text_preserves_raw(name in short_opt_name()) {
        if let Some(opt_name) = OptName::from_text(&name) {
            prop_assert_eq!(opt_name.raw.as_str(), &name);
        }
    }

    #[test]
    fn optname_short_type_detected(c in "[a-zA-Z0-9]") {
        let name = format!("-{}", c);
        if let Some(opt_name) = OptName::from_text(&name) {
            prop_assert_eq!(opt_name.opt_type, OptNameType::ShortType);
        }
    }

    #[test]
    fn optname_long_type_detected(suffix in "[a-z][a-z0-9-]{0,20}") {
        let name = format!("--{}", suffix);
        if let Some(opt_name) = OptName::from_text(&name) {
            prop_assert_eq!(opt_name.opt_type, OptNameType::LongType);
        }
    }

    #[test]
    fn optname_display_equals_raw(name in opt_name_strategy()) {
        prop_assert_eq!(format!("{}", name), name.raw.as_str());
    }
}

// ============================================================================
// Property tests for JSON serialization roundtrip
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(200))]

    #[test]
    fn json_roundtrip_preserves_command(cmd in command_strategy()) {
        let json = JsonGenerator::generate(&cmd);
        let parsed: Command = serde_json::from_str(&json).expect("JSON should parse");

        prop_assert_eq!(&parsed.name, &cmd.name);
        prop_assert_eq!(&parsed.description, &cmd.description);
        prop_assert_eq!(parsed.options.len(), cmd.options.len());

        for (orig, parsed_opt) in cmd.options.iter().zip(parsed.options.iter()) {
            prop_assert_eq!(orig.names.len(), parsed_opt.names.len());
            prop_assert_eq!(&orig.argument, &parsed_opt.argument);
            prop_assert_eq!(&orig.description, &parsed_opt.description);
        }
    }

    #[test]
    fn json_output_is_valid_json(cmd in command_strategy()) {
        let json = JsonGenerator::generate(&cmd);
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("Should be valid JSON");
        prop_assert!(parsed.is_object());
        prop_assert!(parsed.get("name").is_some());
        prop_assert!(parsed.get("options").is_some());
    }
}

// ============================================================================
// Property tests for shell generators
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn bash_generator_produces_valid_output(cmd in command_strategy()) {
        let output = BashGenerator::generate(&cmd);
        // Should contain the function definition or be empty for commands without options
        prop_assert!(output.contains("_") || output.is_empty() || cmd.options.is_empty());
        // Output should be valid UTF-8 (guaranteed since it's a String)
    }

    #[test]
    fn zsh_generator_produces_valid_output(cmd in command_strategy()) {
        let output = ZshGenerator::generate(&cmd);
        // Zsh completions should not panic and produce a string
        // (output format varies based on command structure)
        let _ = output; // Just verify it doesn't panic
    }

    #[test]
    fn fish_generator_produces_valid_output(cmd in command_strategy()) {
        let output = FishGenerator::generate(&cmd);
        // Fish completions use 'complete' command
        if !cmd.options.is_empty() {
            prop_assert!(output.contains("complete") || output.is_empty());
        }
    }

    #[test]
    fn elvish_generator_produces_valid_output(cmd in command_strategy()) {
        let output = ElvishGenerator::generate(&cmd);
        // Elvish completions should produce valid output
        let _ = output; // Just verify it doesn't panic
    }

    #[test]
    fn nushell_generator_produces_valid_output(cmd in command_strategy()) {
        let output = NushellGenerator::generate(&cmd);
        // Nushell completions should produce valid output
        let _ = output; // Just verify it doesn't panic
    }

    #[test]
    fn all_generators_handle_empty_command(_seed in 0u64..1000) {
        let cmd = Command::new(EcoString::from("empty"));

        // None of these should panic
        let _ = BashGenerator::generate(&cmd);
        let _ = ZshGenerator::generate(&cmd);
        let _ = FishGenerator::generate(&cmd);
        let _ = ElvishGenerator::generate(&cmd);
        let _ = NushellGenerator::generate(&cmd);
        let _ = JsonGenerator::generate(&cmd);
    }
}

// ============================================================================
// Property tests for Layout parsing
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(200))]

    #[test]
    fn layout_parse_blockwise_never_panics(content in ".*") {
        // Should never panic on any input
        let _ = Layout::parse_blockwise(&content);
    }

    #[test]
    fn layout_preprocess_blockwise_never_panics(content in ".*") {
        let _ = Layout::preprocess_blockwise(&content);
    }

    #[test]
    fn layout_parse_usage_never_panics(content in ".*") {
        let _ = Layout::parse_usage(&content);
    }

    #[test]
    fn layout_parses_standard_help_format(help in help_text_strategy()) {
        let opts = Layout::parse_blockwise(&help);
        // Should parse without panicking; may find options depending on input format
        // All parsed options should have valid structure
        for opt in &opts {
            prop_assert!(!opt.names.is_empty(), "Parsed option should have at least one name");
        }
    }

    #[test]
    fn parsed_options_have_valid_names(help in help_text_strategy()) {
        let opts = Layout::parse_blockwise(&help);
        for opt in opts {
            for name in &opt.names {
                prop_assert!(name.raw.starts_with('-'), "Option name should start with dash: {}", name.raw);
            }
        }
    }
}

// ============================================================================
// Property tests for Postprocessor
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(200))]

    #[test]
    fn unicode_spaces_to_ascii_is_idempotent(s in ".*") {
        let once = Postprocessor::unicode_spaces_to_ascii(&s);
        let twice = Postprocessor::unicode_spaces_to_ascii(&once);
        prop_assert_eq!(once, twice);
    }

    #[test]
    fn remove_bullets_is_idempotent(s in ".*") {
        let once = Postprocessor::remove_bullets(&s);
        let twice = Postprocessor::remove_bullets(&once);
        prop_assert_eq!(once, twice);
    }

    #[test]
    fn fix_command_preserves_name(cmd in command_strategy()) {
        let fixed = Postprocessor::fix_command(cmd.clone());
        prop_assert_eq!(&fixed.name, &cmd.name);
    }

    #[test]
    fn fix_command_removes_duplicate_options(cmd in command_strategy()) {
        let fixed = Postprocessor::fix_command(cmd);
        // After fix_command, there should be no duplicate (names, argument) pairs
        // The deduplication considers both names and argument
        let mut seen = std::collections::HashSet::new();
        for opt in &fixed.options {
            let key: (Vec<_>, &str) = (
                opt.names.iter().map(|n| n.raw.as_str()).collect(),
                &opt.argument,
            );
            prop_assert!(seen.insert(key), "Duplicate options should be removed");
        }
    }
}

// ============================================================================
// Property tests for edge cases
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn handles_unicode_in_descriptions(desc in "[\\p{L}\\p{N}\\s]{0,50}") {
        let opt = Opt {
            names: eco_vec![OptName::new(EcoString::from("-u"), OptNameType::ShortType)],
            argument: EcoString::new(),
            description: EcoString::from(desc.clone()),
        };
        let cmd = Command {
            name: EcoString::from("unicode-test"),
            description: EcoString::new(),
            usage: EcoString::new(),
            options: eco_vec![opt],
            subcommands: eco_vec![],
            version: EcoString::new(),
        };

        // All generators should handle unicode without panicking
        let _ = BashGenerator::generate(&cmd);
        let _ = ZshGenerator::generate(&cmd);
        let _ = FishGenerator::generate(&cmd);
        let _ = JsonGenerator::generate(&cmd);
    }

    #[test]
    fn handles_special_characters_in_names(suffix in "[a-z0-9._+-]{1,15}") {
        let name = format!("--{}", suffix);
        // Should not panic
        let _ = OptName::from_text(&name);
    }

    #[test]
    fn handles_very_long_descriptions(len in 100usize..1000) {
        let desc = "a".repeat(len);
        let opt = Opt {
            names: eco_vec![OptName::new(EcoString::from("--long-desc"), OptNameType::LongType)],
            argument: EcoString::new(),
            description: EcoString::from(desc),
        };
        let cmd = Command {
            name: EcoString::from("long-test"),
            description: EcoString::new(),
            usage: EcoString::new(),
            options: eco_vec![opt],
            subcommands: eco_vec![],
            version: EcoString::new(),
        };

        // Should handle long descriptions without issues
        let json = JsonGenerator::generate(&cmd);
        prop_assert!(json.len() > len);
    }

    #[test]
    fn handles_many_options(count in 50usize..200) {
        let options: EcoVec<Opt> = (0..count)
            .map(|i| Opt {
                names: eco_vec![OptName::new(EcoString::from(format!("--opt-{}", i)), OptNameType::LongType)],
                argument: EcoString::new(),
                description: EcoString::from(format!("Option {}", i)),
            })
            .collect();

        let cmd = Command {
            name: EcoString::from("many-opts"),
            description: EcoString::new(),
            usage: EcoString::new(),
            options,
            subcommands: eco_vec![],
            version: EcoString::new(),
        };

        // Should handle many options
        let json = JsonGenerator::generate(&cmd);
        let parsed: Command = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(parsed.options.len(), count);
    }

    /// Test that parallel parsing produces consistent results
    #[test]
    fn parallel_parsing_is_deterministic(help in help_text_strategy()) {
        // Parse the same content multiple times
        let opts1 = Layout::parse_blockwise(&help);
        let opts2 = Layout::parse_blockwise(&help);
        let opts3 = Layout::parse_blockwise(&help);

        // Results should be identical (order may vary due to parallelism,
        // but the set of options should be the same)
        prop_assert_eq!(opts1.len(), opts2.len());
        prop_assert_eq!(opts2.len(), opts3.len());

        // Check that all option names are present
        let names1: std::collections::HashSet<_> = opts1.iter()
            .flat_map(|o| o.names.iter().map(|n| n.raw.clone()))
            .collect();
        let names2: std::collections::HashSet<_> = opts2.iter()
            .flat_map(|o| o.names.iter().map(|n| n.raw.clone()))
            .collect();
        let names3: std::collections::HashSet<_> = opts3.iter()
            .flat_map(|o| o.names.iter().map(|n| n.raw.clone()))
            .collect();

        prop_assert_eq!(&names1, &names2);
        prop_assert_eq!(&names2, &names3);
    }
}
