# Contributing to translated-lyrics

## Commit Message Convention

This project uses [Conventional Commits](https://www.conventionalcommits.org/).

### Format

```
type(scope): lowercase description
```

### Rules

- **Types:** `feat`, `fix`, `refactor`, `perf`, `docs`, `style`, `chore`, `ci`, `test`, `lint`
- **Scopes** (optional): `lyrics`, `cli`, `deps`, `readme`, `toolchain`, `test`, or other relevant area
- **Description:** always lowercase after the colon, no trailing period, brief (3-7 words preferred)
- **Breaking changes:** append `!` before the colon (e.g. `feat(cli)!: remove deprecated flag`)
- **Code identifiers** in descriptions should be wrapped in backticks (e.g. `` chore(deps): update `rand` ``)

### Pull Request Titles

Pull request titles must follow the same Conventional Commits format as commit messages, using the same `type(scope): lowercase description` pattern. The title of a squash-merged pull request becomes the commit message on the default branch, so the same rules apply.

### Merge Commits

A merge commit message must also follow the Conventional Commits format. Whenever `git merge` creates a merge commit, such as during a non-fast-forward merge, always supply an explicit message rather than accepting the default `Merge branch '…'` text that Git generates. For example, use `git merge --no-ff -m "chore(merge): integrate feature branch" feature-branch`. This rule is especially important for AI agents, which tend to run `git merge` without reviewing or overriding the default commit message.

## Code Style

Automated tools enforce formatting (`cargo fmt`) and linting (`cargo clippy --all-targets`). The following conventions are **not** enforced by those tools and must be followed manually.

### Variable and Closure Parameter Naming

Use **descriptive names** for variables and closure parameters by default. Single-letter names are permitted only in the specific cases listed below.

#### When single-letter names are allowed

- **Comparison closures:** `|a, b|` in `sort_by`, `cmp`, or similar two-argument comparison callbacks. This is idiomatic Rust.

  ```rust
  items.sort_by(|a, b| a.name.cmp(&b.name));
  ```

- **Conventional single-letter names:** `n` for a natural number (unsigned integer / count); `f` for a `fmt::Formatter`; and similar well-established conventions from math or the Rust standard library. Note: for indices, use `index`, `idx`, or `*_index`, not `n`. (For `i`/`j`/`k`, see the dedicated rule below.)

  ```rust
  fn with_capacity(n: usize) -> Self { todo!() }
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { todo!() }
  ```

- **Index variables (`i`, `j`, `k`):** These may only be used in two contexts: short closures and index-based loops. In all other cases, use `index`, `idx`, or `*_index`.

- **Trivial single-expression closures:** A closure whose body is a single field access, method call, or wrapper may use a single letter when the type and purpose are obvious from context.

  ```rust
  .pipe(|x| vec![x])
  ```

- **Fold accumulators:** `acc` for the accumulator and a single letter for the element in trivial folds.

  ```rust
  .fold(PathBuf::new(), |acc, x| acc.join(x))
  ```

- **Test fixtures:** `let a`, `let b`, `let c` for interchangeable specimens with identical roles in equality or comparison tests. Do not use single letters when the variables have distinct roles; use `actual`/`expected` or similar descriptive names instead.

#### When single-letter names are NOT allowed

- **Multi-line functions and closures:** If a function or closure body spans multiple lines, use a descriptive name.

  ```rust
  // Good
  .map(|entry| {
      let file_name = entry.file_name();
      target_dir.join(file_name)
  })

  // Bad
  .map(|e| {
      let file_name = e.file_name();
      target_dir.join(file_name)
  })
  ```

- **`let` bindings in non-test code:** Always use descriptive names.

  ```rust
  // Good
  let metadata = entry.metadata()?;
  // Bad
  let m = entry.metadata()?;
  ```

- **Function and method parameters:** Always use descriptive names, except for conventional single-letter names listed above (`n`, `f`, etc.).

### Trait Bounds

Prefer `where` clauses over inline bounds when there are multiple constraints:

```rust
impl<Key, Value> MyTrait for MyStruct<Key, Value>
where
    Key: Hash + Eq,
    Value: Clone + Default,
```

### Error Handling

- Minimize `unwrap()` in non-test code; use proper error propagation. `unwrap()` is acceptable in tests and for provably infallible operations (with a comment explaining why). When deliberately ignoring an error, use `.ok()` with a comment explaining why.

### Conditional Test Skipping: `#[cfg]` vs `#[cfg_attr(..., ignore)]`

When a test cannot run under certain conditions, such as on the wrong platform, prefer `#[cfg_attr(..., ignore)]` over `#[cfg(...)]` to skip it. The test still compiles on every configuration and is only skipped at runtime. This approach catches type errors and regressions that a `#[cfg]` skip would hide.

Use `#[cfg]` on tests **only** when the code cannot compile under the condition. An example is a test that uses platform-specific types or functions gated behind `#[cfg]`.

Prefer including a reason string in the `ignore` attribute to explain why the test is skipped.

```rust
// Good: test compiles everywhere, skipped at runtime on non-unix
#[test]
#[cfg_attr(not(unix), ignore = "only unix path separators are tested")]
fn unix_path_logic() { /* uses hardcoded unix paths but no unix-only types */ }

// Good: test CANNOT compile on non-unix (uses unix-only types)
#[cfg(unix)]
#[test]
fn unix_only_types() { /* uses OsStrExt which only exists on unix */ }
```

### Test Module Imports

Test modules appear in two shapes. An inline module lives in the same file as the code under test, introduced by `#[cfg(test)] mod tests { ... }`. An external module lives in a sibling file, declared with `#[cfg(test)] mod tests;` next to a companion `src/<module>/tests.rs`. The rules below apply identically to both; the choice between inline and external placement does not affect the import style.

#### Prefer an explicit brace list over `use super::*;`

Tests should declare which symbols they use. A glob hides the surface area of the module under test, silently absorbs newly added items, and breaks grep for callers of any given symbol.

```rust
// Good: each symbol under test is named.
use super::{CollectionName, ParseCollectionNameError, VideoTitle};

// Avoid: pulls every public item from the parent, including items
// the test never references.
use super::*;
```

A glob is acceptable only when a module intentionally re-exports its own payload for consumers, for example `use super::prelude::*;` where `prelude` is a deliberate internal API. In that case the glob targets the prelude rather than the parent itself.

#### Import each item from its canonical path

When a test needs a symbol that does not live in its direct parent module, import it from the module that defines it rather than through a name the parent happens to bring into its own scope with `use` or `pub use`. In Rust, a plain `use` does not re-export; it introduces a binding in the parent's namespace that a child module can still reference through `super::`. A `pub use` additionally exposes the binding to outside callers. Both forms are fragile dependencies for a test: the canonical path remains valid regardless of how the parent reorganizes its own imports, while the indirect path breaks the moment the parent reshapes its own `use` statements.

```rust
// In `src/foo/tests.rs`, when `SomeType` is defined in `crate::bar`:

// Good: canonical path, stable across parent refactors.
use crate::bar::SomeType;

// Avoid: relies on `src/foo.rs` containing `use crate::bar::SomeType;`
// at the top of the file. Removing or renaming that line in the
// parent silently breaks the test's import.
use super::SomeType;
```

This rule applies whether the parent's binding is a private `use` or a `pub use`, because either kind is often an incidental import rather than part of the module's public contract.

### Using `pipe-trait`

This codebase uses the [`pipe-trait`](https://docs.rs/pipe-trait) crate for method-chaining through unary functions, keeping code in a natural left-to-right reading order. Import it as `use pipe_trait::Pipe;`.

Any callable that takes a single argument works with `.pipe()`. This includes free functions, closures, newtype constructors, enum variant constructors, `Some`, `Ok`, `Err`, and trait methods such as `From::from`.

#### When to use pipe

**Chaining through a unary function at the end of an expression chain:**

```rust
// Good: pipe keeps the chain flowing left-to-right
entry.file_name().pipe(OsStringDisplay::from).pipe(Some)
```

**Avoiding deeply nested function calls:**

```rust
// Nested calls are harder to read
let data = serde_json::from_reader::<_, JsonData>(stdin());

// Prefer piping instead
let data = stdin().pipe(serde_json::from_reader::<_, JsonData>);
```

**Continuing a method chain through a free function and back to methods:**

```rust
// Good: pipe bridges from methods to a free function and back
path_buf
    .pipe_as_ref(fs::read_to_string)
    .map(|content| content.trim().to_owned())
```

#### When NOT to use pipe

**Simple standalone function calls.** Pipe adds noise with no readability benefit:

```rust
// Bad: unnecessary pipe
let result = value.pipe(foo);

// Good: just call the function directly
let result = foo(value);
```

### Using `command-extra`

This codebase uses the [`command-extra`](https://docs.rs/command-extra) crate to build `std::process::Command` values in a chainable, owned style. Import it as `use command_extra::CommandExtra;`.

The standard `Command` builder methods, such as `arg`, `env`, and `current_dir`, take `&mut self` and return `&mut Command`. This makes them unsuitable for method chains that end in an owned value. The `CommandExtra` extension trait provides `.with_*` counterparts that take ownership and return an owned `Command`, enabling fluent one-expression construction:

```rust
// Good: fully chainable, owned style
let output = Command::new("my-tool")
    .with_arg("--flag")
    .with_arg(value)
    .output()
    .expect("spawn my-tool");

// Avoid: mutable-reference style, cannot chain with owned methods
let mut cmd = Command::new("my-tool");
cmd.arg("--flag");
cmd.arg(value);
let output = cmd.output().expect("spawn my-tool");
```

Available `.with_*` methods mirror every standard builder method: `with_arg`, `with_args`, `with_env`, `with_envs`, `with_env_remove`, `with_env_clear`, `with_current_dir`, `with_stdin`, `with_stdout`, `with_stderr`.

### Unicode Escape Codes

Write Unicode characters in string literals as the literal glyph whenever the character is visible in a monospaced editor. The `\u{...}` escape sequence is reserved for characters whose visual form is absent or ambiguous. Every other character belongs in the source as itself, including ASCII, CJK characters, Latin letters with diacritics, accented Cyrillic, Arabic-Indic digits, full-width digits, and full-width punctuation.

Writing a visible character as an escape code has no benefit. It makes the source line harder to read at the call site, harder to search for with the literal character, and indistinguishable at a glance from legitimate escapes for invisible characters. Reviewers learn to skim past `\u{...}` sequences, and that habit lets the genuinely invisible ones slip through.

#### When to keep the escape

Use `\u{...}` only for characters whose glyph is absent, ambiguous, or easily confused with something else. Add an explanatory comment when the context does not make the escape's purpose obvious. Examples:

- `\u{3000}` IDEOGRAPHIC SPACE, which renders as blank space and is visually indistinguishable from the regular `\u{0020}` SPACE.
- `\u{200B}` ZERO WIDTH SPACE and other zero-width characters.
- Combining marks written on their own, outside a grapheme that makes their purpose clear.
- Control characters in the range `\u{0000}` through `\u{001F}`, the delete character `\u{007F}`, and the range `\u{0080}` through `\u{009F}`.

#### Examples

- **Full-width digit.** Write `"００:00.000"` rather than `"\u{FF10}\u{FF10}:00.000"`.
- **Full-width colon.** Write `"role：name"` rather than `"role\u{FF1A}name"`.
- **Ideographic space.** Write `"role：name\u{3000}role：name"` rather than `"role\u{FF1A}name\u{3000}role\u{FF1A}name"`. The full-width colons switch to their literal glyphs because they are visible, while the ideographic space stays escaped because its glyph is not.
- **ASCII digit or letter.** Write `"01"` rather than `"\u{0030}\u{0031}"`.

#### Editor note

Some editors and some chat-style interfaces silently re-escape pasted Unicode characters on save or on copy. When that happens, do not try to type the glyph back in by hand. Use a command-line replacement instead, for example:

```sh
perl -CSD -i -pe 's/\\u\{FF10\}/\x{ff10}/g' path/to/file
```

So far this behavior has only been observed with [Claude Code Web](https://claude.ai/code/).

## Setup

Install the required Rust toolchain and components before running any checks:

```sh
rustup toolchain install "$(< rust-toolchain)"
rustup component add --toolchain "$(< rust-toolchain)" rustfmt clippy
```

To run the spell check locally, install the Node.js dependencies with [pnpm](https://pnpm.io):

```sh
pnpm install --frozen-lockfile
```

## Automated Checks

Before submitting, ensure:

- `cargo fmt -- --check` passes
- `cargo clippy --all-targets` passes
- `cargo test` passes

You can run all of these with:

```sh
cargo fmt -- --check && cargo clippy --all-targets && cargo test
```

> [!IMPORTANT]
> Always run the full Rust test suite before committing, even for seemingly trivial changes such as documentation edits, comment changes, or config updates. Any change can break formatting, linting, or tests.

> [!NOTE]
> If a sync test fails, read its error message carefully and run the exact command it tells you to run.

### Spell Check

Run the [CSpell](https://cspell.org) spell check when a change may introduce new words:

```sh
pnpm exec cspell lint --no-progress --gitignore '**'
```

