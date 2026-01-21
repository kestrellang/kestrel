# Kestrel Test Suite API

## Basic Usage

```rust
use kestrel_test_suite::*;

#[test]
fn example_test() {
    Test::new(r#"
module Test
struct Foo { }
"#)
    .expect(Compiles)
    .expect(Symbol::new("Foo").is(SymbolKind::Struct));
}
```

## Compilation Expectations

```rust
.expect(Compiles)                    // Must compile successfully
.expect(Fails)                       // Must fail (any error)
.expect(HasError("substring"))       // Must have error containing substring
.expect(HasErrorCount(n))            // Must have exactly n errors
.expect(HasWarning("substring"))     // Must have warning containing substring
.expect(NoWarnings)                  // Must have no warnings
```

## Symbol Expectations

```rust
Symbol::new("Name")                  // Find symbol by name
Symbol::new("Outer.Inner")           // Nested lookup (e.g., struct method)
    .is(SymbolKind::Struct)          // Assert kind
    .has(Behavior::Visibility(Visibility::Public))
    .has(Behavior::FieldCount(3))
    .has(Behavior::TypeParamCount(2))
    .has(Behavior::IsGeneric(true))
    .has(Behavior::IsStatic(false))
    .has(Behavior::HasBody(true))
    .has(Behavior::ParameterCount(2))
    .has(Behavior::ConformanceCount(1))
    .has(Behavior::IsInstanceMethod(true))
    .has(Behavior::ReceiverKind(Receiver::Borrowing))
    .has(Behavior::ChildCount(2))
    .not(Behavior::IsStatic(true))   // Negate a check
```

## Symbol Kinds

```rust
SymbolKind::Module
SymbolKind::Struct
SymbolKind::Protocol
SymbolKind::Function
SymbolKind::Field
SymbolKind::TypeParameter
SymbolKind::TypeAlias
SymbolKind::AssociatedType
SymbolKind::Local
SymbolKind::Initializer
```

## Multi-File Tests

```rust
Test::with_files(&[
    ("main.ks", "module Main\nimport Other\n..."),
    ("other.ks", "module Other\npublic struct Foo { }"),
])
.expect(Compiles);
```

## Test Organization

Tests are in `lib/kestrel-test-suite/tests/`:
- `declarations/` - Symbol declarations
- `types/` - Type system
- `expressions/` - Expression resolution
- `statements/` - Statement resolution
- `validation/` - Semantic validation
- `instantiation/` - Instance creation

## Running Tests

```bash
cargo test -p kestrel-test-suite              # All tests
cargo test -p kestrel-test-suite --test NAME  # Specific file
cargo test -p kestrel-test-suite test_name    # Specific test
cargo test -p kestrel-test-suite test_name -- --nocapture  # With output
```
