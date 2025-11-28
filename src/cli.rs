use clap::{Parser, ValueEnum};
use clap_verbosity_flag::Verbosity;

/// Default cache TTL in hours (24 hours)
pub const DEFAULT_CACHE_TTL_HOURS: u64 = 24;

#[derive(ValueEnum, Clone, Debug, Copy)]
pub enum Shell {
    /// Bash shell completion
    Bash,
    /// Fish shell completion
    Fish,
    /// Zsh shell completion
    Zsh,
    /// PowerShell completion
    #[value(name = "powershell")]
    PowerShell,
    /// Elvish shell completion
    Elvish,
    /// Nushell completion
    Nushell,
}

#[derive(Parser, Debug)]
#[command(
    version,
    author,
    about = "Parse help or manpage texts and generate shell completion scripts",
    long_about = "d2o extracts CLI options from help text and exports them as shell completion scripts or JSON."
)]
pub struct Cli {
    /// Extract CLI options from the help texts or man pages associated with the command
    #[arg(
        long,
        short = 'c',
        help = "Extract options from a command's help or man page",
        long_help = "Extract CLI options from the help texts or man pages associated with the command. Subcommand pages are also scanned automatically.",
        conflicts_with_all = ["file", "subcommand", "loadjson"],
    )]
    pub command: Option<String>,

    /// Extract CLI options from a file
    #[arg(
        long,
        short = 'f',
        help = "Extract options from a help text file",
        long_help = "Extract CLI options from a text file containing help or manpage output.",
        conflicts_with_all = ["command", "subcommand", "loadjson"],
    )]
    pub file: Option<String>,

    /// Extract CLI options from a subcommand (format: command-subcommand, e.g., git-log)
    #[arg(
        long,
        short = 's',
        help = "Extract options from a subcommand",
        long_help = "Extract CLI options from a subcommand. The format is command-subcommand (for example: git-log).",
        conflicts_with_all = ["command", "file", "loadjson"],
    )]
    pub subcommand: Option<String>,

    /// Load JSON file in Command schema
    #[arg(
        long,
        short = 'l',
        help = "Load a Command JSON file",
        long_help = "Load a JSON file that uses d2o's Command schema and operate on that instead of parsing help text.",
        conflicts_with_all = ["command", "file", "subcommand"],
    )]
    pub loadjson: Option<String>,

    /// Output format: bash, zsh, fish, json, native, elvish, nushell
    #[arg(
        long,
        short = 'o',
        help = "Select output format",
        long_help = "Select output format: bash, zsh, fish, json, native, elvish, or nushell.",
        value_parser = ["bash", "zsh", "fish", "json", "native", "elvish", "nushell"],
        default_value = "native",
    )]
    pub format: String,

    /// Output in JSON (same as --format=json)
    #[arg(
        long,
        short = 'j',
        help = "Output in JSON (deprecated)",
        long_help = "Output in JSON. This is equivalent to setting --format=json and is kept for legacy compatibility."
    )]
    pub json: bool,

    /// Skip scanning manpage and focus on help text
    #[arg(
        long,
        short = 'm',
        help = "Skip scanning man pages",
        long_help = "Skip scanning man pages and focus only on --help output. This does not apply if the input source is a file."
    )]
    pub skip_man: bool,

    /// List subcommands (debug)
    #[arg(
        long,
        short = 'L',
        help = "List discovered subcommands",
        long_help = "List subcommands discovered from the parsed help text instead of generating completions.",
        conflicts_with = "loadjson"
    )]
    pub list_subcommands: bool,

    /// Run preprocessing only (debug)
    #[arg(
        long,
        short = 'd',
        help = "Run preprocessing only",
        long_help = "Run only the preprocessing phase and print the parsed option/description pairs for debugging.",
        conflicts_with = "loadjson"
    )]
    pub debug: bool,

    /// Set upper bound of the depth of subcommand level
    #[arg(
        long,
        short = 'D',
        help = "Limit subcommand parsing depth",
        long_help = "Set an upper bound on how deeply to scan for nested subcommands.",
        default_value = "4"
    )]
    pub depth: usize,

    /// Generate shell completions
    #[arg(
        long,
        short = 'C',
        value_name = "SHELL",
        help = "Generate shell completion script",
        long_help = "Generate a shell completion script for the given shell (bash, zsh, fish, powershell, elvish, nushell)."
    )]
    pub completions: Option<Shell>,

    /// Write completion script to RC file (~/.bashrc, ~/.zshrc, etc.)
    /// Automatically detects shell and appends to appropriate rc file
    #[arg(
        long,
        short = 'w',
        help = "Write output to shell RC file",
        long_help = "Write the generated completion script to the appropriate shell RC file (for example, ~/.bashrc or ~/.zshrc) instead of printing it to stdout."
    )]
    pub write: bool,

    /// Use bash-completion extended format for bash output
    /// (encodes descriptions as name:Description and calls __ltrim_colon_completions if available)
    #[arg(
        long,
        short = 'b',
        help = "Use bash-completion extended format",
        long_help = "Use bash-completion's extended format for bash output. This encodes descriptions as name:Description and calls __ltrim_colon_completions if available."
    )]
    pub bash_completion_compat: bool,

    /// Enable caching of parsed commands (default: enabled)
    #[arg(
        long,
        help = "Enable caching of parsed commands",
        long_help = "Enable caching of parsed Command objects. Cached entries are stored in the XDG cache directory and reused if the help text hasn't changed and TTL hasn't expired.",
        default_value = "true",
        action = clap::ArgAction::Set,
        value_parser = clap::value_parser!(bool),
    )]
    pub cache: bool,

    /// Cache TTL in hours (default: 24)
    #[arg(
        long,
        help = "Set cache TTL in hours",
        long_help = "Set the time-to-live for cache entries in hours. Entries older than this are considered stale and will be re-parsed.",
        default_value_t = DEFAULT_CACHE_TTL_HOURS,
        value_name = "HOURS",
    )]
    pub cache_ttl: u64,

    /// Clear all cached entries
    #[arg(
        long,
        help = "Clear all cache entries",
        long_help = "Remove all cached Command entries from the cache directory."
    )]
    pub cache_clear: bool,

    /// Show cache statistics
    #[arg(
        long,
        help = "Show cache statistics",
        long_help = "Display statistics about the cache including number of entries, sizes, and location."
    )]
    pub cache_stats: bool,

    /// Set the level of verbosity (-v, -vv, -q, etc.)
    #[command(flatten)]
    pub verbosity: Verbosity,
}

impl Cli {
    /// Get the effective format, considering --json flag as legacy
    pub fn effective_format(&self) -> &str {
        if self.json { "json" } else { &self.format }
    }

    /// Get the input file/command, prioritizing loadjson
    pub fn get_input(&self) -> Option<&str> {
        self.loadjson
            .as_deref()
            .or(self.file.as_deref())
            .or(self.command.as_deref())
    }

    /// Check if preprocess only mode (renamed from debug for clarity)
    pub fn is_preprocess_only(&self) -> bool {
        self.debug
    }
}
