# AI Instructions

Read and follow the CONTRIBUTING.md file in this repository for all code style conventions, commit message format, and development guidelines.

## Quick Reference

- Commit format: Conventional Commits. Pattern: `type(scope): lowercase description`.
- Write documentation, comments, and other prose in a direct, formal tone using complete sentences. Avoid mid-sentence breaks introduced by em dashes, long parenthetical clauses, or conversational asides such as "basically", "just", "that'd be slow", or "etc." Em dashes are a reliable symptom of loose phrasing; when one appears, restructure the surrounding sentence so each clause stands on its own rather than swapping the em dash for another punctuation mark.
- Use descriptive names for variables and closure parameters. Single letters are permitted only in the following cases: conventional names such as `n` for a count and `f` for a formatter; two-argument comparison closures written as `|a, b|`; trivial single-expression closures; fold accumulators; index variables `i`, `j`, and `k` in short closures or index-based loops; and test fixtures with identical roles. Single letters are never permitted in multi-line functions or closures.
- Use `pipe-trait` to chain through unary functions such as constructors, `Some`, `Ok`, and free functions. Use it to flatten nested calls and to continue method chains. Do not use it for simple standalone calls; prefer `foo(value)` over `value.pipe(foo)`.
- Use the `command-extra` crate (the `CommandExtra` trait) when building `std::process::Command`. Call `.with_arg(...)`, `.with_env(...)`, and similar methods rather than `.arg(...)` or `.env(...)`, so construction remains a single owned expression chain.
- Prefer `where` clauses when a type has multiple trait bounds.
- Minimize `unwrap()` in non-test code. Use proper error handling instead.
- Prefer `#[cfg_attr(..., ignore = "reason")]` over `#[cfg(...)]` when skipping tests. Use `#[cfg]` on tests only when the code cannot compile under the condition, such as when it references types or functions that do not exist on other platforms.
- Install the toolchain before running tests: `rustup toolchain install "$(< rust-toolchain)" && rustup component add --toolchain "$(< rust-toolchain)" rustfmt clippy`.
- Validate changes with `cargo fmt -- --check && cargo clippy --all-targets && cargo test`.
- **Always run the full Rust test suite** (`cargo fmt -- --check && cargo clippy --all-targets && cargo test`) before every commit. This rule applies to all changes, including documentation changes, comment edits, and config updates.
- When a sync test fails, read its error message and run the exact command it reports.
- Run the CSpell spell check when a change may introduce new words: `pnpm install --frozen-lockfile && pnpm exec cspell lint --gitignore '**'`.
- When the user provides a diff to apply, run `git apply` rather than interpreting each hunk manually. When a diff is provided for context or discussion, respond accordingly.
