# TODO

## High Priority (Future Work)

- [x] Add support for elvish and nushell in format
- [x] Add --write flag for caching completions to ~/.hcl (cli.rs) (used with -c just caches in ~/.hcl the output)
- [ ] Implement rc file append behavior for --write (auto-append to .bashrc/.zshrc/config.fish)
- [x] Add more snapshot regression tests for generators and parser
- [ ] Reach 90%+ coverage in cargo llvm-cov
- [ ] Full h2o compatibility (100%) (https://github.com/yamaton/h2o.git)
- [ ] Implement --write functionality to auto-append to .bashrc/.zshrc/.config/fish/config.fish
- [ ] Caching mechanism with TTL (cache parsed help for N hours)
- [ ] Performance optimization for 100+ MB help text files
- [ ] Parallel parsing for multiple commands using rayon
- [ ] Future roff parser (separate crate)
- [ ] Async framework (tokio)
- [ ] Asyncification
- [ ] `ecow`, `scc`, `memchr` etc etc crates (more crates for perf)
- [ ] Future convert one completion format to another

## Medium Priority

- [x] Add -f short flag support for --file (if original supported)
- [ ] Property-based testing with proptest
- [ ] Stress tests with massive help text files
- [ ] Performance benchmarking suite with criterion

## Nice to Have

- [ ] Plugin system for custom parsers
- [ ] Interactive mode for testing completions
- [ ] Completion validation checker
- [x] Shell-specific optimizations (bash/zsh completions improved)

## Research Items

- [x] Verify if original h2o supported -f shorthand
- [x] Check bioinformatics tool parsing edge cases (covered by dedicated parser test)
- [x] Evaluate roff parser complexity vs. benefit (decision: keep future roff parser as separate crate; defer implementation)
