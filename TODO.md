# TODO

## High Priority (Future Work)

- [x] Add support for elvish and nushell in format
- [x] Add --write flag for caching completions to ~/.hcl (cli.rs) (used with -c just caches in ~/.hcl the output)
- [x] Finalize --write design: user adds `source "$(hcl -c fzf --format zsh --write)"` (or similar) in rc files; hcl never auto-appends to rc files
- [x] Add more snapshot regression tests for generators and parser
- [x] Reach 90%+ coverage in cargo llvm-cov
- [x] Full h2o compatibility (100%) (https://github.com/yamaton/h2o.git)
- [x] Document --write usage pattern for rc files (no auto-append; user-managed source lines)
- [x] Caching mechanism with TTL (cache parsed help for N hours)
- [ ] Performance optimization for 100+ MB help text files
- [x] Parallel parsing for multiple commands using rayon
- [ ] Future roff parser (separate crate)
- [ ] Async framework (tokio)
- [ ] Asyncification
- [ ] `ecow`, `scc`, `memchr` etc etc crates (more crates for perf)
- [ ] Future convert one completion format to another

## Medium Priority

- [x] Add -f short flag support for --file (if original supported)
- [x] Property-based testing with proptest
- [ ] Stress tests with massive help text files
- [x] Performance benchmarking suite with criterion (using divan)

## Nice to Have

- [ ] Plugin system for custom parsers
- [ ] Interactive mode for testing completions
- [ ] Completion validation checker
- [x] Shell-specific optimizations (bash/zsh completions improved)

## Research Items

- [x] Verify if original h2o supported -f shorthand
- [x] Check bioinformatics tool parsing edge cases (covered by dedicated parser test)
- [x] Evaluate roff parser complexity vs. benefit (decision: keep future roff parser as separate crate; defer implementation)
