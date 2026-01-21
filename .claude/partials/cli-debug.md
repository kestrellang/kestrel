# CLI Debugging Commands

## Syntax and Parsing

```bash
# Parse and show syntax tree (CST)
cargo run -- parse file.ks --tree
```

## Semantic Analysis

```bash
# Show symbol table
cargo run -- file.ks --symbols

# Show semantic tree (summary)
cargo run -- file.ks --tree

# Show semantic tree (detailed with function bodies)
cargo run -- file.ks --tree=full
```

## Code Generation

```bash
# Show execution graph (MIR)
cargo run -- file.ks --xgraph
```

## Full Debug Output

```bash
# Everything at once
cargo run -- file.ks --tree=full --symbols --xgraph
```

## LLDB Debugging

```bash
# Build with debug symbols
cargo build

# Run with lldb
lldb target/debug/kestrel -- file.ks

# Common lldb commands
b kestrel_semantic_tree_binder::body_resolver::resolve_expr  # Set breakpoint
r                                                              # Run
n                                                              # Next line
s                                                              # Step into
p variable_name                                                # Print variable
bt                                                             # Backtrace
```

## Useful Breakpoint Locations

| Issue Type | Breakpoint Location |
|------------|---------------------|
| Expression resolution | `kestrel_semantic_tree_binder::body_resolver` |
| Type resolution | `kestrel_semantic_tree_binder::resolution::type_resolver` |
| Symbol creation | `kestrel_semantic_tree_builder::builders` |
| Validation errors | `kestrel_semantic_analyzers` |

## Running Tests with Debug Output

```bash
# Run specific test with output
cargo test -p kestrel-test-suite test_name -- --nocapture

# Run with RUST_BACKTRACE
RUST_BACKTRACE=1 cargo test -p kestrel-test-suite test_name -- --nocapture
```
