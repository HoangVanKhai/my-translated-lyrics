# AI Instructions

Read and follow the CONTRIBUTING.md file in this repository for all code style conventions, commit message format, and development guidelines.

## Quick Reference

- Commit format: Conventional Commits — `type(scope): lowercase description`
- Use descriptive variable and closure parameter names by default — single letters are only allowed in: conventional names (`n` for count, `f` for formatter), comparison closures (`|a, b|`), trivial single-expression closures, fold accumulators, index variables (`i`/`j`/`k` in short closures or index-based loops only), and test fixtures (identical roles only). Never use single letters in multi-line functions or closures
- Use `pipe-trait` for chaining through unary functions (constructors, `Some`, `Ok`, free functions, etc.), avoiding nested calls, and continuing method chains — but not for simple standalone calls (prefer `foo(value)` over `value.pipe(foo)`)
- Prefer `where` clauses for multiple trait bounds
- Minimize `unwrap()` in non-test code — use proper error handling
- Prefer `#[cfg_attr(..., ignore = "reason")]` over `#[cfg(...)]` to skip tests — use `#[cfg]` on tests only when the code cannot compile under the condition (e.g., references types/functions that don't exist on other platforms)
- Install toolchain before running tests: `rustup toolchain install "$(< rust-toolchain)" && rustup component add --toolchain "$(< rust-toolchain)" rustfmt clippy`
- Run `cargo fmt -- --check && cargo clippy --all-targets && cargo test` to validate changes
- **ALWAYS run the full test suite** (`cargo fmt -- --check && cargo clippy --all-targets && cargo test`) before committing, regardless of how trivial the change seems — this includes documentation-only changes, comment edits, and config changes
- If a sync test fails, read its error message carefully and run the exact command it tells you to run
- When the user provides a diff and you need to update the files, don't manually interpret each hunk (that'd be slow); apply it with `git apply` instead. If the user provides a diff for context or discussion rather than as a change to apply, respond accordingly instead.
