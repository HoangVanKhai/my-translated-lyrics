# AI Instructions

Read and follow the CONTRIBUTING.md file in this repository for all code style conventions, commit message format, and development guidelines.

## Quick Reference

- Commit format: Conventional Commits. Pattern: `type(scope): lowercase description`. The scope is optional.
- Pull request titles must follow the same Conventional Commits convention as commit messages, using the `type(scope): lowercase description` pattern.
- Merge commit messages must also follow the Conventional Commits convention. Whenever `git merge` creates a merge commit, such as during a non-fast-forward merge, supply an explicit message with `-m "type(scope): lowercase description"` rather than accepting Git's default `Merge branch '…'` text.
- Write documentation, comments, and other prose for ease of understanding first. Prefer a formal tone when it does not hurt clarity, and use complete sentences. Avoid mid-sentence breaks introduced by em dashes or long parenthetical clauses. Em dashes are a reliable symptom of loose phrasing; when one appears, restructure the surrounding sentence so each clause stands on its own rather than swapping the em dash for another punctuation mark.
- Write Unicode characters in string literals as the literal glyph whenever the character is visible in a monospaced editor. This includes ASCII, CJK, Latin letters with diacritics, accented Cyrillic, Arabic-Indic digits, full-width digits, and full-width punctuation. Reserve `\u{...}` escapes for characters whose glyph is absent or ambiguous, such as `\u{3000}` IDEOGRAPHIC SPACE, `\u{200B}` ZERO WIDTH SPACE and other zero-width characters, standalone combining marks, and control characters. Add an explanatory comment when the escape's purpose is not obvious from context.
- Use `pipe-trait` to chain through unary functions such as constructors, `Some`, `Ok`, and free functions. Use it to flatten nested calls and to continue method chains. Do not use it for simple standalone calls; prefer `foo(value)` over `value.pipe(foo)`.
- Use the `command-extra` crate (the `CommandExtra` trait) when building `std::process::Command`. Call `.with_arg(...)`, `.with_env(...)`, and similar methods rather than `.arg(...)` or `.env(...)`, so construction remains a single owned expression chain.
- Prefer `where` clauses when a type has multiple trait bounds.
- Minimize `unwrap()` in non-test code. Use proper error handling instead.
- Write leading-prefix parsers in the parser-combinator style: name the function `take`, return the unconsumed tail with the value (`Result<(T, &str), E>`, `Option<(T, &str)>`, or `(&str, &str)`), and follow parse-don't-validate by keeping shape-mismatch errors separate from range errors. Use this for layered grammars, not where a single regex or `serde`/`toml` already applies. See the Parser Combinators section in CONTRIBUTING.md.
- Prefer `#[cfg_attr(..., ignore = "reason")]` over `#[cfg(...)]` when skipping tests. Use `#[cfg]` on tests only when the code cannot compile under the condition, such as when it references types or functions that do not exist on other platforms.
- In test modules, prefer explicit brace lists such as `use super::{Foo, Bar};` over `use super::*;` so each symbol under test is declared. Import items that live outside the direct parent module by their canonical path (for example, `use crate::bar::SomeType;`) rather than through a name the parent happens to bring into its own scope with `use` or `pub use`. These rules apply equally to inline `#[cfg(test)] mod tests { ... }` blocks and external `src/<module>/tests.rs` files.
- Install the toolchain before running tests: `rustup toolchain install "$(< rust-toolchain)" && rustup component add --toolchain "$(< rust-toolchain)" rustfmt clippy`.
- Validate changes with `cargo fmt -- --check && cargo clippy --workspace --all-targets && cargo test --workspace`.
- **Always run the full Rust test suite** (`cargo fmt -- --check && cargo clippy --workspace --all-targets && cargo test --workspace`) before every commit. This rule applies to all changes, including documentation changes, comment edits, and config updates.
- When a sync test fails, read its error message and run the exact command it reports.
- Run the CSpell spell check when a change may introduce new words: `pnpm install --frozen-lockfile && pnpm exec cspell lint --gitignore '**'`.
- When the user provides a diff to apply, run `git apply` rather than interpreting each hunk manually. When a diff is provided for context or discussion, respond accordingly.
- The `gh` (GitHub CLI) is not installed. Do not attempt to use it.
- After completing a task, rewrite the pull request description so that it summarizes the totality of the pull request's changes in their final state. Do not leave the description as a running bullet list of checkboxes or AI task updates.
