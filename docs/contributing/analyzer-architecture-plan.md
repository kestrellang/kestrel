# Semantic Analyzer Architecture Plan

This document outlines the plan to extract validation passes from `kestrel-semantic-tree-builder` into a new `kestrel-semantic-analyzers` crate, and move the query layer (`SemanticDatabase`) to `kestrel-semantic-tree`.

## Goals

1. **Separation of concerns** - Builder builds, analyzers analyze
2. **Reusability** - Analysis can run on any semantic tree, not just ones from the builder
3. **Clean layering** - No circular dependencies, clear responsibilities

## New Crate Structure

### Dependency Graph

```
kestrel-semantic-tree
        ↑
kestrel-semantic-analyzers
        ↑
kestrel-semantic-tree-builder
```

### `kestrel-semantic-tree`

The semantic tree crate gains the database/query layer. This makes sense because querying a semantic tree is fundamental to working with it.

**Dependencies:** `semantic-tree`, `kestrel-span`, `kestrel-prelude`

**New modules:**
```
src/
  database/
    mod.rs
    registry.rs           # SymbolRegistry
    extension_registry.rs # ExtensionRegistry
    queries.rs            # Db trait, SymbolResolution, TypePathResolution, etc.
    semantic_db.rs        # SemanticDatabase
  visibility.rs           # VisibilityChecker
  traversal.rs            # find_ancestor_of_kind, find_visibility_scope
```

### `kestrel-semantic-analyzers`

New crate for read-only analysis passes over the semantic tree.

**Dependencies:** `kestrel-semantic-tree`, `kestrel-reporting`, `kestrel-span`

**Structure:**
```
src/
  lib.rs                  # pub use analyzer, run, run_all
  analyzer.rs             # Analyzer trait
  context.rs              # AnalysisContext
  runner.rs               # run(), run_all() free functions
  walker.rs               # internal tree walking
  diagnostics/            # error types (moved from builder)
  analyzers/              # implementations (moved from builder's validation/)
    mod.rs
    type_check.rs
    dead_code.rs
    conformance.rs
    struct_cycles.rs
    type_alias_cycles.rs
    constraint_cycles.rs
    duplicate_symbol.rs
    exhaustive_return.rs
    extension_conflict.rs
    function_body.rs
    generics.rs
    imports.rs
    initializer_verification.rs
    protocol_method.rs
    static_context.rs
    assignment.rs
    visibility_consistency.rs
```

### `kestrel-semantic-tree-builder`

Remains focused on construction and binding.

**Dependencies:** `kestrel-semantic-tree`, `kestrel-semantic-analyzers`, `kestrel-syntax-tree`, `kestrel-reporting`

**Removed modules:**
- `database/` (moved to `kestrel-semantic-tree`)
- `validation/` (moved to `kestrel-semantic-analyzers`)
- `diagnostics/` (moved to `kestrel-semantic-analyzers`)
- `resolution/visibility.rs` (moved to `kestrel-semantic-tree`)

## Analyzer API Design

### Analyzer Trait

```rust
pub trait Analyzer {
    fn name(&self) -> &'static str;

    // Pre-visit hooks (called before children)
    fn visit_symbol(&mut self, symbol: &Arc<dyn Symbol<KestrelLanguage>>, ctx: &mut AnalysisContext) {}
    fn visit_statement(&mut self, stmt: &Statement, ctx: &mut AnalysisContext) {}
    fn visit_expression(&mut self, expr: &Expression, ctx: &mut AnalysisContext) {}
    fn visit_type(&mut self, ty: &Ty, ctx: &mut AnalysisContext) {}
    fn visit_pattern(&mut self, pattern: &Pattern, ctx: &mut AnalysisContext) {}

    // Post-visit hooks (called after children)
    fn visit_symbol_post(&mut self, symbol: &Arc<dyn Symbol<KestrelLanguage>>, ctx: &mut AnalysisContext) {}
    fn visit_statement_post(&mut self, stmt: &Statement, ctx: &mut AnalysisContext) {}
    fn visit_expression_post(&mut self, expr: &Expression, ctx: &mut AnalysisContext) {}
    fn visit_type_post(&mut self, ty: &Ty, ctx: &mut AnalysisContext) {}
    fn visit_pattern_post(&mut self, pattern: &Pattern, ctx: &mut AnalysisContext) {}

    /// Called after walk completes (even if stopped early)
    fn finalize(&mut self, ctx: &mut AnalysisContext) {}
}
```

### Analysis Context

Single unified context type with control flow methods:

```rust
pub struct AnalysisContext<'a> {
    pub db: &'a SemanticDatabase,
    pub diagnostics: &'a mut DiagnosticContext,
    file_id: usize,      // tracked by walker
    stopped: bool,
    skip: bool,
}

impl AnalysisContext<'_> {
    /// Report a diagnostic
    pub fn report(&mut self, error: impl IntoDiagnostic) {
        self.diagnostics.throw(error, self.file_id);
    }

    /// Stop the entire walk immediately
    pub fn stop(&mut self) {
        self.stopped = true;
    }

    /// Skip children of the current node
    pub fn skip_children(&mut self) {
        self.skip = true;
    }
}
```

### Runner API

Free functions that create the context internally:

```rust
/// Run a single analyzer
pub fn run<A: Analyzer>(
    analyzer: &mut A,
    root: &Arc<dyn Symbol<KestrelLanguage>>,
    db: &SemanticDatabase,
    diagnostics: &mut DiagnosticContext,
);

/// Run multiple analyzers in a single walk
pub fn run_all(
    analyzers: &mut [&mut dyn Analyzer],
    root: &Arc<dyn Symbol<KestrelLanguage>>,
    db: &SemanticDatabase,
    diagnostics: &mut DiagnosticContext,
);
```

### Control Flow Semantics

- **`ctx.stop()`** - Stops the walk immediately. No post-visit hooks run. `finalize()` still runs.
- **`ctx.skip_children()`** - Skips children of current node. Post-visit for current node still runs.
- When running multiple analyzers, each tracks its own stop/skip state independently.

## Migration Steps

### Phase 1: Move Database to `kestrel-semantic-tree`

1. Create `database/` module in `kestrel-semantic-tree`
2. Move `SymbolRegistry` from builder
3. Move `ExtensionRegistry` from builder
4. Move `Db` trait and query types from builder
5. Move `SemanticDatabase` from builder
6. Create `visibility.rs` with `VisibilityChecker`
7. Create `traversal.rs` with `find_ancestor_of_kind`
8. Update builder to import from `kestrel-semantic-tree`

### Phase 2: Create `kestrel-semantic-analyzers`

1. Create new crate with Cargo.toml
2. Implement `Analyzer` trait
3. Implement `AnalysisContext`
4. Implement walker (internal)
5. Implement `run()` and `run_all()`

### Phase 3: Move Diagnostics

1. Move `diagnostics/` module from builder to analyzers crate
2. Update imports

### Phase 4: Convert Validators to Analyzers

1. Move each validator from `validation/` to `analyzers/`
2. Rename `Validator` implementations to `Analyzer`
3. Update method signatures (`validate_*` -> `visit_*`)
4. Change return type from nothing to using `ctx.stop()`/`ctx.skip_children()`
5. Update `finalize` signature

### Phase 5: Wire Up Builder

1. Remove old validation module from builder
2. Add dependency on `kestrel-semantic-analyzers`
3. Update build/bind pipeline to call analyzers

## Example Analyzer Implementation

```rust
pub struct DeadCodeAnalyzer {
    warnings: Vec<UnreachableCodeWarning>,
}

impl DeadCodeAnalyzer {
    pub fn new() -> Self {
        Self { warnings: Vec::new() }
    }
}

impl Analyzer for DeadCodeAnalyzer {
    fn name(&self) -> &'static str {
        "dead_code"
    }

    fn visit_symbol(
        &mut self,
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        ctx: &mut AnalysisContext,
    ) {
        let kind = symbol.metadata().kind();
        if !matches!(kind, KestrelSymbolKind::Function | KestrelSymbolKind::Initializer) {
            return;
        }

        if let Some(body) = get_executable_body(symbol) {
            analyze_block(&body.statements, body.yield_expr.as_deref(), &mut self.warnings);
        }
    }

    fn finalize(&mut self, ctx: &mut AnalysisContext) {
        for warning in self.warnings.drain(..) {
            ctx.report(warning);
        }
    }
}
```

## Usage After Migration

```rust
use kestrel_semantic_tree::database::SemanticDatabase;
use kestrel_semantic_analyzers::{run_all, analyzers::*};

// After building and binding...
let db = SemanticDatabase::new(registry);

// Run all standard analyzers
let mut analyzers: Vec<&mut dyn Analyzer> = vec![
    &mut TypeCheckAnalyzer::new(),
    &mut DeadCodeAnalyzer::new(),
    &mut ConformanceAnalyzer::new(),
    // ...
];

kestrel_semantic_analyzers::run_all(
    &mut analyzers,
    tree.root(),
    &db,
    &mut diagnostics,
);
```
