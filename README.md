# `d2o`

![Build Status](https://img.shields.io/badge/build-passing-brightgreen)
![Version](https://img.shields.io/badge/version-0.1.0-blue)
![License](https://img.shields.io/badge/license-MIT-green)

A high-performance, pure Rust rewrite of h2o - a CLI tool that extracts command-line options from help text and man pages, then exports them as shell completion scripts or JSON.

## Installation

### From Source

```bash
cd /home/muntasir/projects/d2o
cargo build --release
./target/release/d2o --help
```

The release binary will be at `target/release/d2o`

### Pre-compiled Binary

Copy the binary to your PATH:

```bash
cp target/release/d2o ~/.local/bin/
```

### Using Cargo

```bash
cargo install d2o
```

### Using `cargo-binstall`

```bash
cargo binstall d2o
```

Completions are also available for `bash`, `zsh`, `fish`, `powershell`, `elvish` and `nushell`:

```bash
d2o --completions <shell> # Replace <shell> with your shell name
```

Manpages are in tarballs and zips in releases.

## Usage

### Generate shell completion

```shell
# Generate fish completion script from `man ls` or `ls --help`
d2o --command ls --format fish > ls.fish

# Generate zsh completion script
d2o --command git --format zsh > git.zsh

# Generate bash completion script (plain options only)
d2o --command docker --format bash > docker.bash

# Generate bash completion script compatible with bash-completion (includes descriptions)
d2o --command docker --format bash --bash-completion-compat > docker.bash
```

### Export as JSON

```shell
# Export CLI info as JSON
d2o --command ls --format json

# Pretty-print JSON output
d2o --command curl --format json | jq .
```

### Parse local file

```shell
# Save man page to file first
man grep | col -bx > grep.txt

# Parse from file
d2o --file grep.txt --format fish > grep.fish
```

### Advanced options

```shell
# Skip man page lookup (use --help only)
d2o --command cargo --skip-man --format json

# List subcommands
d2o --command git --list-subcommands

# Extract subcommand options
d2o --subcommand git-log --format fish

# Preprocess only (debug option splitting)
d2o --command ls --preprocess-only

# Scan deeper for nested subcommands
d2o --command docker --depth 2 --format json
```

### Building

```bash
# Debug build
cargo build

# Release build with optimizations
cargo build --release
```

### Running with verbose output

```bash
RUST_LOG=debug ./target/release/d2o --command ls --format json
```

## Known Limitations

- Subcommand depth must be specified (default: 1 level)
- Some highly unusual help text formats may not parse perfectly
- Unicode box-drawing characters in help text are converted to ASCII

## Related Projects

- [parse-help](https://github.com/sindresorhus/parse-help)

## License

MIT - See [LICENSE](./LICENSE) file

## Contributing

Contributions welcome! Areas for improvement:

- Performance optimizations
- Additional help text format support
- More comprehensive testing
- Documentation
