# `hcl`

![Build Status](https://img.shields.io/badge/build-passing-brightgreen)
![Version](https://img.shields.io/badge/version-0.1.0-blue)
![License](https://img.shields.io/badge/license-MIT-green)

A high-performance, pure Rust rewrite of h2o - a CLI tool that extracts command-line options from help text and man pages, then exports them as shell completion scripts or JSON.

## Installation

### From Source

```bash
cd /home/muntasir/projects/hcl
cargo build --release
./target/release/hcl --help
```

The release binary will be at `target/release/hcl`

### Pre-compiled Binary

Copy the binary to your PATH:

```bash
cp target/release/hcl ~/.local/bin/
```

### Using Cargo

```bash
cargo install hcl
```

### Using `cargo-binstall`

```bash
cargo binstall hcl
```

Completions are also available for `bash`, `zsh`, `fish`, `powershell`, `elvish` and `nushell`:

```bash
hcl --completions <shell> # Replace <shell> with your shell name
```

Manpages are in tarballs and zips in releases.

## Usage

### Generate shell completion

```shell
# Generate fish completion script from `man ls` or `ls --help`
hcl --command ls --format fish > ls.fish

# Generate zsh completion script
hcl --command git --format zsh > git.zsh

# Generate bash completion script
hcl --command docker --format bash > docker.bash
```

### Export as JSON

```shell
# Export CLI info as JSON
hcl --command ls --format json

# Pretty-print JSON output
hcl --command curl --format json | jq .
```

### Parse local file

```shell
# Save man page to file first
man grep | col -bx > grep.txt

# Parse from file
hcl --file grep.txt --format fish > grep.fish
```

### Advanced options

```shell
# Skip man page lookup (use --help only)
hcl --command cargo --skip-man --format json

# List subcommands
hcl --command git --list-subcommands

# Extract subcommand options
hcl --subcommand git-log --format fish

# Preprocess only (debug option splitting)
hcl --command ls --preprocess-only

# Scan deeper for nested subcommands
hcl --command docker --depth 2 --format json
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
RUST_LOG=debug ./target/release/hcl --command ls --format json
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
