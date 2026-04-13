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

## Code Style

Automated tools enforce formatting (`cargo fmt`) and linting (`cargo clippy --all-targets`). The following conventions are **not** enforced by those tools and must be followed manually.

### Variable and Closure Parameter Naming

Use **descriptive names** for variables and closure parameters by default. Single-letter names are permitted only in the specific cases listed below.

#### When single-letter names are allowed

- **Comparison closures:** `|a, b|` in `sort_by`, `cmp`, or similar two-argument comparison callbacks — this is idiomatic Rust.

  ```rust
  items.sort_by(|a, b| a.name.cmp(&b.name));
  ```

- **Conventional single-letter names:** `n` for a natural number (unsigned integer / count), `f` for a `fmt::Formatter`, and similar well-established conventions from math or the Rust standard library. Note: for indices, use `index`, `idx`, or `*_index` — not `n`. (For `i`/`j`/`k`, see the dedicated rule below.)

  ```rust
  fn with_capacity(n: usize) -> Self { todo!() }
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { todo!() }
  ```

- **Index variables (`i`, `j`, `k`):** These may only be used in two contexts: (1) short closures, and (2) index-based loops/iterations (rare in Rust). In all other cases, use `index`, `idx`, or `*_index`.

- **Trivial single-expression closures:** A closure whose body is a single field access, method call, or wrapper may use a single letter when the type and purpose are obvious from context.

  ```rust
  .pipe(|x| vec![x])
  ```

- **Fold accumulators:** `acc` for the accumulator and a single letter for the element in trivial folds.

  ```rust
  .fold(PathBuf::new(), |acc, x| acc.join(x))
  ```

- **Test fixtures:** `let a`, `let b`, `let c` for interchangeable specimens with identical roles in equality or comparison tests. Do not use single letters when the variables have distinct roles — use `actual`/`expected` or similar descriptive names instead.

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

- Minimize `unwrap()` in non-test code — use proper error propagation. `unwrap()` is acceptable in tests and for provably infallible operations (with a comment explaining why). When deliberately ignoring an error, use `.ok()` with a comment explaining why.

### Conditional Test Skipping: `#[cfg]` vs `#[cfg_attr(..., ignore)]`

When a test cannot run under certain conditions (e.g., wrong platform), prefer `#[cfg_attr(..., ignore)]` over `#[cfg(...)]` to skip it. This way the test is still compiled on all configurations — catching type errors and regressions early — but simply skipped at runtime.

Use `#[cfg]` on tests **only** when the code cannot compile under the condition — for example, when the test references types, functions, or trait methods that are gated behind `#[cfg]` and do not exist on other platforms.

Prefer including a reason string in the `ignore` attribute to explain why the test is skipped.

```rust
// Good — test compiles everywhere, skipped at runtime on non-unix
#[test]
#[cfg_attr(not(unix), ignore = "only unix path separators are tested")]
fn unix_path_logic() { /* uses hardcoded unix paths but no unix-only types */ }

// Good — test CANNOT compile on non-unix (uses unix-only types)
#[cfg(unix)]
#[test]
fn unix_only_types() { /* uses OsStrExt which only exists on unix */ }
```

### Using `pipe-trait`

This codebase uses the [`pipe-trait`](https://docs.rs/pipe-trait) crate for method-chaining through unary functions, keeping code in a natural left-to-right reading order. Import it as `use pipe_trait::Pipe;`.

Any callable that takes a single argument works with `.pipe()` — free functions, closures, newtype constructors, enum variant constructors, `Some`, `Ok`, `Err`, trait methods like `From::from`, etc.

#### When to use pipe

**Chaining through a unary function at the end of an expression chain:**

```rust
// Good — pipe keeps the chain flowing left-to-right
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
// Good — pipe bridges from methods to a free function and back
path_buf
    .pipe_as_ref(fs::read_to_string)
    .map(|content| content.trim().to_owned())
```

#### When NOT to use pipe

**Simple standalone function calls** — pipe adds noise with no readability benefit:

```rust
// Bad — unnecessary pipe
let result = value.pipe(foo);

// Good — just call the function directly
let result = foo(value);
```

## Setup

Install the required Rust toolchain and components before running any checks:

```sh
rustup toolchain install "$(< rust-toolchain)"
rustup component add --toolchain "$(< rust-toolchain)" rustfmt clippy
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
> Always run the full test suite before committing, even for seemingly trivial changes such as documentation edits, comment changes, or config updates. Any change can break formatting, linting, or tests.

> [!NOTE]
> If a sync test fails, read its error message carefully and run the exact command it tells you to run.
