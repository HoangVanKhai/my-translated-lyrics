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

A merge commit message must also follow the Conventional Commits format. Always supply an explicit message when running `git merge`, for example `git merge --no-ff -m "chore(merge): integrate feature branch" feature-branch`, rather than accepting the default `Merge branch '…'` message that Git generates. This rule is especially important for AI agents, which tend to run `git merge` without reviewing or overriding the default commit message.

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

