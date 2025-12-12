# Semantic Analyzer Architecture Plan

This document outlines the plan to extract validation passes from `kestrel-semantic-tree-builder` into a new `kestrel-semantic-analyzers` crate.

## Current State

The semantic model layer has already been extracted:

- **`kestrel-semantic-model`** - Contains `SemanticModel`, `Query` trait, registries, resolution types
- **`kestrel-span`** - `Span` now includes `file_id`, eliminating need to pass it separately
- **`kestrel-reporting`** - `IntoDiagnostic::into_diagnostic()` no longer requires `file_id` parameter

## Goals

1. **Separation of concerns** - Builder builds, analyzers analyze
2. **Reusability** - Analysis can run on any semantic model, not just ones from the builder
3. **Clean layering** - No circular dependencies, clear responsibilities

## Crate Structure

### Dependency Graph

```
kestrel-semantic-tree
        ↑
kestrel-semantic-model
        ↑
kestrel-semantic-analyzers
        ↑
kestrel-semantic-tree-builder
```

### `kestrel-semantic-analyzers`

New crate for read-only analysis passes over the semantic model.

**Dependencies:** `kestrel-semantic-model`, `kestrel-semantic-tree`, `kestrel-reporting`, `kestrel-span`

**Structure:**
```
src/
  lib.rs                  # pub use analyzer, run, run_all
  analyzer.rs             # Analyzer trait
  context.rs              # AnalysisContext
  runner.rs               # run(), run_all() free functions
  walker.rs               # internal tree walking
  diagnostics/            # error types (moved from builder)
    mod.rs
    type_check.rs
    control_flow.rs
    cycles.rs
    ...
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
    type_assignability.rs
```

### `kestrel-semantic-tree-builder`

After migration, focused on construction and binding only.

**Removed modules:**
- `validation/` (moved to `kestrel-semantic-analyzers`)
- `diagnostics/` (moved to `kestrel-semantic-analyzers`)

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

Single unified context type. Since `Span` now includes `file_id`, diagnostics can be reported directly without tracking file context:

```rust
pub struct AnalysisContext<'a> {
    pub model: &'a SemanticModel,
    pub diagnostics: &'a mut DiagnosticContext,
    stopped: bool,
    skip: bool,
}

impl AnalysisContext<'_> {
    /// Report a diagnostic (file_id comes from the error's span)
    pub fn report(&mut self, error: impl IntoDiagnostic) {
        self.diagnostics.throw(error);
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
    model: &SemanticModel,
    diagnostics: &mut DiagnosticContext,
);

/// Run multiple analyzers in a single walk
pub fn run_all(
    analyzers: &mut [&mut dyn Analyzer],
    model: &SemanticModel,
    diagnostics: &mut DiagnosticContext,
);
```

### Control Flow Semantics

- **`ctx.stop()`** - Stops the walk immediately. No post-visit hooks run. `finalize()` still runs.
- **`ctx.skip_children()`** - Skips children of current node. Post-visit for current node still runs.
- When running multiple analyzers, each tracks its own stop/skip state independently.

## Migration Steps

### Phase 1: Create `kestrel-semantic-analyzers` Crate

1. Create new crate with Cargo.toml
2. Implement `Analyzer` trait
3. Implement `AnalysisContext`
4. Implement walker (internal)
5. Implement `run()` and `run_all()`

### Phase 2: Move Diagnostics

1. Move `diagnostics/` module from builder to analyzers crate
2. Update error types to use `Span.file_id` directly in `into_diagnostic()`
3. Update imports throughout

### Phase 3: Convert Validators to Analyzers

For each validator in `validation/`:

1. Move to `analyzers/` in new crate
2. Rename from `*Validator` to `*Analyzer`
3. Change trait impl from `Validator` to `Analyzer`
4. Update method signatures:
   - `validate_symbol` -> `visit_symbol`
   - `validate_expression` -> `visit_expression`
   - etc.
5. Replace `ctx.diagnostics().get().throw(error, file_id)` with `ctx.report(error)`
6. Replace `ctx.db` / `ctx.model` references with `ctx.model`

### Phase 4: Move Support Code

1. Move `type_assignability.rs` to analyzers crate
2. Move any shared utilities used only by analyzers

### Phase 5: Wire Up Builder

1. Remove old `validation/` module from builder
2. Remove old `diagnostics/` module from builder
3. Add dependency on `kestrel-semantic-analyzers`
4. Update build/bind pipeline to call analyzers

## Example Analyzer Implementation

```rust
use kestrel_semantic_analyzers::{Analyzer, AnalysisContext};
use kestrel_semantic_model::SymbolFor;

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

## Example: Using Queries in Analyzers

```rust
impl Analyzer for TypeCheckAnalyzer {
    fn visit_expression(&mut self, expr: &Expression, ctx: &mut AnalysisContext) {
        match &expr.kind {
            ExprKind::SymbolRef(symbol_id) => {
                // Use query-based lookup
                if let Some(symbol) = ctx.model.query(SymbolFor { id: *symbol_id }) {
                    // ... analyze
                }
            }
            // ...
        }
    }
}
```

## Usage After Migration

```rust
use kestrel_semantic_model::SemanticModel;
use kestrel_semantic_analyzers::{run_all, analyzers::*};

// After building and binding...
let model = SemanticBinder::bind(tree, &mut diagnostics);

// Run all standard analyzers
let mut analyzers: Vec<&mut dyn Analyzer> = vec![
    &mut TypeCheckAnalyzer::new(),
    &mut DeadCodeAnalyzer::new(),
    &mut ConformanceAnalyzer::new(),
    &mut StructCycleAnalyzer::new(),
    // ...
];

kestrel_semantic_analyzers::run_all(
    &mut analyzers,
    &model,
    &mut diagnostics,
);
```

## Files to Move

### From `kestrel-semantic-tree-builder/src/diagnostics/` to `kestrel-semantic-analyzers/src/diagnostics/`

- `mod.rs`
- `type_check.rs`
- `control_flow.rs`
- `cycles.rs`
- `call.rs`
- `declaration.rs`
- `member_access.rs`
- `module.rs`
- `operators.rs`
- `protocol.rs`
- `struct_init.rs`
- `type_resolution.rs`
- `visibility.rs`
- `assignment.rs`

### From `kestrel-semantic-tree-builder/src/validation/` to `kestrel-semantic-analyzers/src/analyzers/`

- `type_check.rs`
- `dead_code.rs`
- `conformance.rs`
- `struct_cycles.rs`
- `type_alias_cycles.rs`
- `constraint_cycles.rs`
- `duplicate_symbol.rs`
- `exhaustive_return.rs`
- `extension_conflict.rs`
- `function_body.rs`
- `generics.rs`
- `imports.rs`
- `initializer_verification.rs`
- `protocol_method.rs`
- `static_context.rs`
- `assignment_validation.rs`
- `visibility_consistency.rs`
- `type_assignability.rs`
