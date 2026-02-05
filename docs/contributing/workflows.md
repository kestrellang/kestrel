# Common Workflows

Step-by-step guides for common development tasks.

## Adding a New Language Feature

This is the most common workflow. Use this when adding new syntax like a keyword, declaration type, or expression.

### Files Changed (typical)

Based on git history analysis of features like `self` parameter and member access:

| Phase | Files | Lines Changed |
|-------|-------|---------------|
| Lexer | 1 file | ~5-10 |
| Parser | 3-5 files | ~100-300 |
| Syntax Tree | 1 file | ~20-50 |
| Semantic Tree | 2-4 files | ~50-200 |
| Semantic Build (lowering) | 1-3 files | ~50-250 |
| Semantic Bind | 1-5 files | ~100-500 |
| Semantic Analyze | 0-3 files | ~0-200 |
| Tests | 1-2 files | ~50-200 |

### Step-by-Step

#### 1. Add Token (if new keyword)
**File**: `lib/kestrel-lexer/src/lib.rs`

```rust
// Add in correct category (alphabetical within category)
// Declaration Keywords section:
#[token("newkeyword")]
NewKeyword,
```

#### 2. Add SyntaxKind Variants
**File**: `lib/kestrel-syntax-tree/src/lib.rs`

```rust
pub enum SyntaxKind {
    // Token (if added above)
    NewKeyword,

    // Syntax nodes
    NewFeatureDeclaration,
    NewFeatureBody,  // if applicable
}
```

Update `kind_from_raw`:
```rust
const NEW_FEATURE_DECLARATION: u16 = SyntaxKind::NewFeatureDeclaration as u16;

match raw.0 {
    NEW_FEATURE_DECLARATION => SyntaxKind::NewFeatureDeclaration,
    // ...
}
```

#### 3. Create Parser
**File**: `lib/kestrel-parser/src/{feature}/mod.rs` (new file)

Follow the event-driven parser pattern:
1. Internal Chumsky parser
2. Emit function
3. Public parse function

**File**: `lib/kestrel-parser/src/lib.rs`
```rust
pub mod newfeature;
pub use newfeature::{NewFeatureDeclaration, parse_newfeature_declaration};
```

#### 4. Integrate into Declaration Items
**File**: `lib/kestrel-parser/src/declaration_item/mod.rs`

**Critical**: Add to `declaration_item_parser_internal()` - without this, the feature won't parse!

```rust
let newfeature_parser = /* ... */
    .map(|data| DeclarationItemData::NewFeature(data));

// Add to the .or() chain
module_parser.or(import_parser).or(struct_parser).or(newfeature_parser)
```

#### 5. Add Symbol Kind
**File**: `lib/kestrel-semantic-tree/src/symbol/kind.rs`
```rust
pub enum KestrelSymbolKind {
    NewFeature,
}
```

#### 6. Create Symbol
**File**: `lib/kestrel-semantic-tree/src/symbol/newfeature.rs` (new file)

```rust
pub struct NewFeatureSymbol {
    metadata: SymbolMetadata<KestrelLanguage>,
}

impl Symbol<KestrelLanguage> for NewFeatureSymbol {
    fn metadata(&self) -> &SymbolMetadata<KestrelLanguage> {
        &self.metadata
    }
}
```

Update `lib/kestrel-semantic-tree/src/symbol/mod.rs`:
```rust
mod newfeature;
pub use newfeature::NewFeatureSymbol;
```

#### 7. Create Builder (BUILD)
**File**: `lib/kestrel-semantic-tree-builder/src/builders/newfeature.rs` (new file)

Implement the `Builder` trait (creates symbols + stores syntax map entries).

Update `lib/kestrel-semantic-tree-builder/src/builders/mod.rs`:
```rust
mod newfeature;
pub use newfeature::NewFeatureBuilder;
```

Register it in `lib/kestrel-semantic-tree-builder/src/lowerer.rs` by:
- adding a `static NEWFEATURE: NewFeatureBuilder = NewFeatureBuilder;`
- extending `builder_for(...)` to return `Some(&NEWFEATURE)` for your `SyntaxKind`

#### 8. Create Binder (BIND)
**File**: `lib/kestrel-semantic-tree-binder/src/binders/newfeature.rs` (new file)

Implement `DeclarationBinder::bind_declaration(...)` for the new symbol kind.

Update `lib/kestrel-semantic-tree-binder/src/binders/mod.rs`:
```rust
mod newfeature;
pub use newfeature::NewFeatureBinder;
```

Register it in `lib/kestrel-semantic-tree-binder/src/declaration_binder.rs` in `DeclarationBinderRegistry::new()`:
```rust
binders.insert(SyntaxKind::NewFeatureDeclaration, Box::new(NewFeatureBinder));
```

#### 9. Add Tests
**File**: `lib/kestrel-test-suite/tests/newfeature.rs` (new file)

```rust
use kestrel_test_suite::{Test, Compiles, HasError, Symbol, SymbolKind};

#[test]
fn basic_newfeature() {
    Test::new("module Main\nnewfeature Foo { }")
        .expect(Compiles)
        .expect(Symbol::new("Foo").is(SymbolKind::NewFeature));
}
```

#### 10. Verify
```bash
cargo test -p kestrel-lexer
cargo test -p kestrel-parser
cargo test -p kestrel-semantic-tree-builder
cargo test -p kestrel-semantic-tree-binder
cargo test -p kestrel-semantic-analyzers
cargo test -p kestrel-test-suite
cargo test
```

---

## Adding Expression/Statement Support

When adding new expression or statement types (e.g., binary operators, if expressions).

### Key Files
- `lib/kestrel-parser/src/expr/mod.rs` - Expression parsing
- `lib/kestrel-parser/src/stmt/mod.rs` - Statement parsing
- `lib/kestrel-semantic-tree/src/expr.rs` - Expression semantics
- `lib/kestrel-semantic-tree/src/stmt.rs` - Statement semantics
- `lib/kestrel-semantic-tree-binder/src/body_resolver/mod.rs` - **Main file**

### Step-by-Step

#### 1. Add SyntaxKind
```rust
// In kestrel-syntax-tree/src/lib.rs
BinaryExpr,  // or IfExpr, WhileStmt, etc.
```

#### 2. Update Parser
Add parsing logic in `expr/mod.rs` or `stmt/mod.rs`.

#### 3. Add Semantic Representation
```rust
// In kestrel-semantic-tree/src/expr.rs
pub enum Expr {
    Binary { left: Box<Expr>, op: BinaryOp, right: Box<Expr> },
    // ...
}
```

#### 4. Update Body Resolver
**File**: `lib/kestrel-semantic-tree-binder/src/body_resolver/mod.rs`

This is where most of the work happens. Add a match arm to handle the new syntax:

```rust
fn resolve_expr(&mut self, node: &SyntaxNode) -> Option<Expr> {
    match node.kind() {
        SyntaxKind::BinaryExpr => self.resolve_binary_expr(node),
        // ...
    }
}

fn resolve_binary_expr(&mut self, node: &SyntaxNode) -> Option<Expr> {
    // Extract operands and operator
    // Resolve types
    // Return Expr::Binary { ... }
}
```

#### 5. Add Diagnostics (if needed)
**File**: `lib/kestrel-semantic-tree-binder/src/diagnostics/{name}.rs`

Create a new diagnostics module for feature-specific errors.

#### 6. Add Tests
**File**: `lib/kestrel-test-suite/tests/body_resolution.rs`

```rust
mod binary_ops {
    #[test]
    fn add_integers() {
        Test::new("module Main\nfunc f() -> Int { 1 + 2 }")
            .expect(Compiles);
    }
}
```

---

## Adding a Validation Pass

When adding semantic checks that run after binding (e.g., checking for invalid modifiers).

### Key Files
- `lib/kestrel-semantic-analyzers/src/analyzers/{name}/mod.rs` (new)
- `lib/kestrel-semantic-analyzers/src/analyzers/mod.rs`
- `lib/kestrel-semantic-analyzers/src/lib.rs` (register in `default_analyzers()`)
- `lib/kestrel-test-suite/tests/validation.rs`

### Step-by-Step

#### 1. Define Errors
Document what errors the pass will detect:

| Condition | Error Message |
|-----------|---------------|
| When X | "error: X happened" |

#### 2. Create Analyzer File
**File**: `lib/kestrel-semantic-analyzers/src/analyzers/mycheck/mod.rs`

```rust
pub struct MyCheckAnalyzer;

impl Analyzer for MyCheckAnalyzer {
    fn name(&self) -> &'static str { "my_check" }
}
```

#### 3. Register Analyzer
Add it to `default_analyzers()` in `lib/kestrel-semantic-analyzers/src/lib.rs` (in the right order).

```rust
// lib/kestrel-semantic-analyzers/src/analyzers/mod.rs
pub mod mycheck;
pub use mycheck::MyCheckAnalyzer;

// lib/kestrel-semantic-analyzers/src/lib.rs
pub fn default_analyzers() -> Vec<Box<dyn Analyzer>> {
    vec![
        // ... existing analyzers
        Box::new(MyCheckAnalyzer),
    ]
}
```

#### 4. Add Tests
**File**: `lib/kestrel-test-suite/tests/validation.rs`

```rust
mod my_check {
    #[test]
    fn valid_case() {
        Test::new("module Main\n/* valid code */")
            .expect(Compiles);
    }

    #[test]
    fn invalid_case() {
        Test::new("module Main\n/* invalid code */")
            .expect(HasError("expected error message"));
    }
}
```

---

## Adding a New Diagnostic

When adding error messages for semantic analysis.

### Key Files
- BIND-time diagnostics: `lib/kestrel-semantic-tree-binder/src/diagnostics/{name}.rs` (new)
- Analyzer diagnostics: `lib/kestrel-semantic-analyzers/src/analyzers/{name}/diagnostics.rs` (new)

### Step-by-Step

#### 1. Create Diagnostic Module
**File**: `lib/kestrel-semantic-tree-binder/src/diagnostics/myerror.rs`

```rust
use kestrel_reporting::{Diagnostic, DiagnosticContext, Label};
use kestrel_span::Span;

pub fn report_my_error(
    diagnostics: &mut DiagnosticContext,
    span: Span,
    name: &str,
) {
    let diagnostic = Diagnostic::error()
        .with_message(format!("my error: '{}'", name))
        .with_labels(vec![
            Label::primary(span.file_id, span.range())
                .with_message("error occurred here")
        ]);

    diagnostics.add_diagnostic(diagnostic);
}
```

#### 2. Export
**File**: `lib/kestrel-semantic-tree-binder/src/diagnostics/mod.rs`

```rust
mod myerror;
pub use myerror::report_my_error;
```

#### 3. Use in Binder or Body Resolver
```rust
use crate::diagnostics::report_my_error;

// When error condition is detected:
report_my_error(diagnostics, span, name);
```

---

## Adding or Modifying Standard Library Methods

The standard library lives in `lang/std/` as Kestrel source files.

### Directory Structure

```
lang/std/
├── collections/    # Array, Set, Dictionary
├── core/           # Bool, Equatable, Comparable, Cloneable, Hash, Range, etc.
├── ffi/            # Foreign function interface
├── io/             # File, stdin/stdout/stderr, Read/Write protocols
├── iter/           # Iterator protocol and adapters
├── memory/         # Layout, Pointer, Slice, RawPointer, SystemAllocator, RcBox
├── num/            # Int64, Float, RandomNumberGenerator
├── result/         # Optional, Result
└── text/           # String, Formattable, FormatOptions
```

### Stdlib File Structure

Each `.ks` file follows this pattern:

```kestrel
module std.collections

import std.core.(Bool, Equatable, Comparable)
import std.num.(Int64)
// ... other imports

public struct MyType[T] {
    private var data: Pointer[T]

    public init() { ... }

    public func myMethod() -> Bool { ... }

    public mutating func myMutatingMethod() { ... }
}
```

Key conventions:
- Module path matches directory structure (`std.collections`, `std.core`, etc.)
- Public API uses `public` visibility
- Internal fields use `private`
- Mutating methods are marked `mutating`
- COW types (String, Array) call `makeUnique()` before `grow()` in mutating methods

### Testing Stdlib Methods

Stdlib tests live in `lib/kestrel-test-suite/tests/stdlib/`. Each test compiles and **runs** a Kestrel program that exercises the method.

```rust
use kestrel_test_suite::*;

#[test]
fn my_method_test() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // Setup
            let x = ...

            // Test the method — return non-zero on failure
            if x.myMethod() == false { return 1 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}
```

Key patterns:
- Use `.with_stdlib()` to include the standard library
- Use `.expect(Runs)` to compile and execute (not just compile)
- `main()` returns `lang.i64` — return `0` for success, non-zero for failure
- Each check returns a unique non-zero value to identify which assertion failed

### Step-by-Step

#### 1. Implement the method
Edit the appropriate file in `lang/std/`. Follow existing patterns in that file.

#### 2. Add tests
Add tests in `lib/kestrel-test-suite/tests/stdlib/{type}.rs`. If the file doesn't exist, create it and add the module to `lib/kestrel-test-suite/tests/stdlib/mod.rs`.

#### 3. Run the specific test
```bash
cargo test -p kestrel-test-suite --release -- test_name
```

---

## Debugging Semantic Resolution Issues

When symbols aren't being created or resolved correctly.

### Diagnostic Steps

#### 1. Check Parser Output
Add debug output to see if syntax tree is correct:
```rust
// In parser test
let tree = TreeBuilder::new(source, sink.into_events()).build();
println!("{:#?}", tree);
```

#### 2. Check Registration
Verify the builder/binder is registered:
```rust
// BUILD: SyntaxKind -> Builder (in builder_for(...) in lowerer.rs)
// BIND: SyntaxKind -> DeclarationBinder (DeclarationBinderRegistry::new)
```

#### 3. Check Symbol Creation
Add debug output in your builder:
```rust
fn build_declaration(&self, syntax: &SyntaxNode, ...) -> Option<...> {
    println!("Building: {:?}", syntax.kind());
    // ...
    println!("Created symbol: {:?}", symbol.metadata().name());
}
```

#### 4. Check Parent-Child Links
```rust
if let Some(parent) = parent {
    parent.metadata().add_child(&symbol_arc);
    println!("Added to parent: {:?}", parent.metadata().name());
}
```

#### 5. Use Test Expectations
```rust
Test::new("module Main\nyour code")
    .expect(Compiles)
    .expect(Symbol::new("YourSymbol").is(SymbolKind::YourKind));
```

If the test fails, it shows what symbols actually exist.

---

## Git Workflow

See [Git](git.md) for the full branching strategy, PR requirements, and issue workflow.

### Commit Messages
```
feature: description of feature
fix: description of bug fix
refactor: description of refactoring
docs: description of documentation change
test: description of test addition
```

### Feature Commits
Features are typically done in a single commit including:
- Lexer changes
- Parser changes
- Syntax tree changes
- Semantic tree changes
- Builder changes
- Tests

### Running Before Commit
```bash
cargo fmt
cargo clippy
cargo test
```
