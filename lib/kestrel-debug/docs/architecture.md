# kestrel-debug Architecture

Category-based debug tracing for the Kestrel compiler. Enables selective debug output from compiler subsystems via environment variables, with zero cost when disabled.

## Usage

```bash
# Single category
KESTREL_DEBUG=infer cargo test

# Multiple categories
KESTREL_DEBUG=infer,hir-lower cargo test

# Everything
KESTREL_DEBUG=all cargo test
```

## Core Types

| Type | Description |
|------|-------------|
| `DebugConfig` | Parsed config: `all` flag + list of enabled category names |
| `config()` | Lazy-initializes from `KESTREL_DEBUG` env var (thread-safe via `OnceLock`) |
| `is_enabled(category)` | Returns `true` if category is active or `all` is set |
| `ktrace!(category, fmt, ...)` | Category-filtered `eprintln!` with `[category]` prefix |

## `ktrace!` Macro

The primary API. Emits to stderr only when the category is enabled:

```rust
ktrace!("infer", "solving constraint {}: {:?}", idx, constraint);
// Output: [infer] solving constraint 42: Equal(ty0, ty1)
```

The `if` guard prevents format string evaluation when disabled, so there is no runtime cost for inactive categories.

## Debug Categories

Categories are free-form string literals chosen at each call site; there is
no registry. List the ones in use:

```bash
rg -oUN --no-filename -r '$1' 'ktrace!\(\s*"([^"]+)"' lib | sort -u
```

## Module Map

| File | Responsibility |
|------|---------------|
| `lib.rs` | `DebugConfig`, `config()`, `is_enabled()`, `ktrace!` — all in one file |

## Dependencies

None — uses only `std::sync::OnceLock`.
