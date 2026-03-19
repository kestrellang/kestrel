# kestrel-analyze — Agent Guide

## Analyzer File Structure

Every analyzer file MUST follow this structure:

### 1. Doc Comment (required)

The file starts with a doc comment describing the analyzer, followed by a section for each diagnostic it produces. Each diagnostic section documents the ID, name, severity, category, message, labels (with span sources), and notes.

```rust
//! # <Analyzer Name>
//!
//! <Brief description of what this analyzer checks.>
//!
//! ## Diagnostics
//!
//! ### <ID> — `<name>` (<Severity>, <Category>)
//!
//! **Message:** "<message template, use {name} for interpolated values>"
//!
//! **Labels:**
//! - Primary: <what this label points to>
//!   - Span source: <which util function and what HIR node it's called on>
//!   - Message: "<label message>"
//! - Secondary: <what this label points to> (if any)
//!   - Span source: <which util function and what HIR node/entity>
//!   - Message: "<label message>"
//!
//! **Notes:**
//! - "<note text>" (or "(none)" if no notes)
```

#### Span Source Documentation

Each label's span source MUST specify:
- Which `util::` function extracts the span (`util::expr_span`, `util::stmt_span`, `util::pat_span`, `util::entity_name`)
- What HIR node or entity it's called on (e.g., "the unreachable `HirStmtId`", "the `HirExprId` of the assignment target")
- For declaration spans: whether it's the **usage site**, the **declaration name**, or the **declaration signature**

Examples of good span source documentation:
```
- Span source: `util::stmt_span` on the last `HirStmtId` in the function body
- Span source: `util::expr_span` on the assignment target `HirExprId`
- Span source: `util::pat_span` on the refutable `HirPatId` in the let binding
- Span source: entity span from `Callable` component on the protocol method declaration (name span)
```

### 2. Descriptor Statics

```rust
static DESCRIPTORS: &[DiagnosticDescriptor] = &[
    DiagnosticDescriptor {
        id: "KS<NNN>",
        name: "<snake_case_name>",
        default_severity: Severity::<Error|Warning|Info>,
        category: Category::<Correctness|Style|Performance|Usage>,
    },
    // Add more descriptors if the analyzer produces multiple diagnostic kinds
];
```

### 3. Analyzer Struct (ZST)

```rust
pub struct <Name>Analyzer;
```

### 4. Trait Implementations

```rust
impl Describe for <Name>Analyzer {
    fn id(&self) -> &'static str { "<analyzer_id>" }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] { DESCRIPTORS }
}

impl BodyCheck for <Name>Analyzer {  // or DeclCheck or CompilationCheck
    fn check(&self, cx: &BodyContext<'_>) -> Vec<AnalyzeDiagnostic> {
        // Pure analysis logic
    }
}
```

### 5. Private Helper Functions

Analysis-specific logic (e.g., divergence checking, control flow analysis) lives as private functions in the analyzer file. Only **span extraction and entity info helpers** go in `util.rs`.

### 6. Registration

Add the analyzer to `default_analyzers()` in `lib.rs`:
```rust
pub fn default_analyzers() -> AnalyzerRegistry {
    let mut r = AnalyzerRegistry::new();
    r.add_body_check(MyNewAnalyzer);  // ← add here
    r
}
```

## Shared Utilities (`util.rs`)

Use these utilities in all analyzers. **Do not create local span extraction or entity name helpers — use the shared ones.** If you need a new utility, add it to `util.rs` and update this table.

### Span Extraction

| Function | Input | Returns | Description |
|----------|-------|---------|-------------|
| `util::expr_span(hir, id)` | `&HirBody, HirExprId` | `Span` | Span of any `HirExpr` variant |
| `util::stmt_span(hir, id)` | `&HirBody, HirStmtId` | `Span` | Span of any `HirStmt` variant |
| `util::pat_span(hir, id)` | `&HirBody, HirPatId` | `Span` | Span of any `HirPat` variant |

### Entity Info

| Function | Input | Returns | Description |
|----------|-------|---------|-------------|
| `util::entity_name(ctx, entity)` | `&QueryContext, Entity` | `String` | Name from `Name` component, or `"<anonymous>"` |

## Diagnostic ID Allocation

IDs follow the pattern `KS<NNN>`:
- **KS001–KS099**: Control flow (exhaustive return, dead code, guard-let divergence)
- **KS100–KS199**: Type checking (branch mismatch, condition not bool, argument type)
- **KS200–KS299**: Mutability and assignment (immutable assignment, captured variable)
- **KS300–KS399**: Patterns (exhaustiveness, refutable in let, irrefutable in match)
- **KS400–KS499**: Declarations (conformance, duplicates, cycles, visibility)
- **KS500–KS599**: Memory semantics (use-after-move, cloneable fields)
- **KS600–KS699**: Functions and closures (missing body, wrong arity, FFI safety)

Current allocations:
- KS001: `missing_return` (exhaustive_return.rs)
- KS002: `unreachable_code` (dead_code.rs)

## Key Conventions

- Analyzers are **stateless ZSTs** — no fields, no mutable state
- Use `cx.query` to read ECS components (`NodeKind`, `Name`, `Callable`, `TypeAnnotation`, etc.)
- Use `cx.hir` to iterate the HIR body, `cx.typed` for resolved types
- Return `Vec<AnalyzeDiagnostic>` — the framework handles accumulation and memoization
- Use `DESCRIPTORS[N].id` and `DESCRIPTORS[N].default_severity` when constructing diagnostics
- Prefer early returns for inapplicable entities (wrong NodeKind, no return type, empty body, etc.)
