# Contributing to d2o

Thank you for your interest in contributing to d2o! This document provides guidelines and instructions for contributing.

## Code of Conduct

This project adheres to the Contributor Covenant [Code of Conduct](./CODE_OF_CONDUCT.md). By participating, you are expected to uphold this code. Please report unacceptable behavior to our security team.

## Getting Started

### Prerequisites

- Rust 1.85 or later
- Cargo
- Git

### Development Setup

1. Fork the repository

1. Clone your fork:

   ```bash
   git clone https://github.com/MuntasirSZN/d2o.git
   cd d2o
   ```

1. Add upstream remote:

   ```bash
   git remote add upstream https://github.com/MuntasirSZN/d2o.git
   ```

1. Create a development branch:

   ```bash
   git checkout -b feature/your-feature-name
   ```

### Building

```bash
# Development build
cargo build

# Release build
cargo build --release

# Run tests
cargo test

# Check formatting
cargo fmt --check

# Run clippy
cargo clippy -- -D warnings
```

## Making Changes

### Code Style

- Follow Rust conventions (use `cargo fmt`)
- Run `cargo clippy` to catch common mistakes
- Write descriptive commit messages
- Keep functions focused and reasonably sized
- Add comments for complex logic

### Testing

- Add unit tests for new functionality
- Ensure all existing tests pass: `cargo test`
- Test with real commands: `d2o --command <cmd> --format json`
- Include edge cases in tests

Example test structure:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_descriptive_name() {
        let input = "test data";
        let result = function_under_test(input);
        assert_eq!(result, expected_value);
    }
}
```

### Documentation

- Update README.md if adding user-facing features
- Add rustdoc comments to public APIs:
  ```rust
  /// Parses a help text and extracts options.
  ///
  /// # Arguments
  /// * `content` - The help text to parse
  ///
  /// # Returns
  /// A vector of parsed options
  pub fn parse_help(content: &str) -> Vec<Opt> {
      // implementation
  }
  ```

## Commit Guidelines

- Use clear, descriptive commit messages
- Reference issues when applicable: `Fixes #123`
- Keep commits focused on a single change
- Use [conventional commits](https://conventionalcommits.org) when possible:
  ```
  feat: add new parser for tool X
  fix: correct option deduplication bug
  docs: update API documentation
  test: add coverage for layout detection
  perf: optimize regex compilation
  ```

## Pull Requests

### Before Submitting

1. Update your branch with upstream:

   ```bash
   git fetch upstream
   git rebase upstream/main
   ```

1. Run the full test suite:

   ```bash
   cargo test
   cargo clippy -- -D warnings
   cargo fmt -- --check
   ```

1. Build the release binary:

   ```bash
   cargo build --release
   ```

### PR Description Template

```markdown
## Description
Brief description of changes

## Type of Change
- [ ] Bug fix
- [ ] New feature
- [ ] Performance improvement
- [ ] Documentation update
- [ ] Breaking change

## Related Issues
Fixes #(issue number)

## Testing
- [ ] Added/updated tests
- [ ] All tests pass
- [ ] Tested with real commands

## Checklist
- [ ] Code follows style guidelines
- [ ] Documentation updated
- [ ] No new warnings from clippy
- [ ] Changes are backward compatible
```

## Areas for Contribution

See [TODO.md](./TODO.md) for a list of features and improvements needed.

## Performance Considerations

When optimizing:

1. Measure before/after with real commands
1. Use `cargo flamegraph` for profiling
1. Test memory usage with large help texts
1. Consider regex complexity
1. Benchmark against reference implementation

Example:

```bash
# Build with profiling
cargo build --release
# Profile a command
perf record ./target/release/d2o --command ls --format json
perf report
```

## Documentation

### Updating README

- Keep it concise and practical
- Include examples for common use cases
- Link to detailed docs in separate files
- Update version numbers consistently

### Writing Docs

- Use clear, simple language
- Provide code examples
- Explain the "why" not just the "what"
- Include edge cases and limitations

## Release Process

The maintainers follow this process:

1. Update version in Cargo.toml
1. Update CHANGELOG.md
1. Create git tag: `git tag v0.x.y`
1. Push tag: `git push origin v0.x.y`
1. GitHub Actions automatically builds and publishes binaries
1. Create release notes on GitHub

## Security Issues

**Do not** open a public issue for security vulnerabilities.

See [SECURITY.md](./SECURITY.md) for responsible disclosure procedures.

## Questions?

- Open a discussion on GitHub
- Check existing issues first
- Ask on related project pages
- See the implementation guide in IMPLEMENTATION.md

## Recognition

Contributors will be recognized in:

- GitHub contributors page
- Release notes (for significant contributions)
- Project documentation (for major features)

## License

By contributing, you agree that your contributions will be licensed under the MIT License.

______________________________________________________________________

Thank you for helping make d2o better! 🚀
