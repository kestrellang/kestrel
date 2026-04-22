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
        id: "E<NNN>",
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

IDs follow the pattern `E<NNN>`:
- **E001–E099**: Control flow (exhaustive return, dead code, guard-let divergence)
- **E100–E199**: Type checking (branch mismatch, condition not bool, argument type)
- **E200–E299**: Mutability and assignment (immutable assignment, captured variable)
- **E300–E399**: Patterns (exhaustiveness, refutable in let, irrefutable in match)
- **E400–E499**: Declarations (conformance, duplicates, cycles, visibility)
- **E500–E599**: Memory semantics (use-after-move, cloneable fields)
- **E600–E699**: Functions and closures (missing body, wrong arity, FFI safety)
- **E700–E799**: Literals and lexing (escape sequences, malformed literals)

Current allocations:
- E001: `missing_return` (exhaustive_return.rs)
- E002: `unreachable_code` (dead_code.rs)
- E301: `refutable_for_loop_pattern` (for_loop_pattern.rs)
- E302: `irrefutable_if_let` (exhaustiveness.rs)
- E303: `irrefutable_match_arm` (exhaustiveness.rs) — reserved; currently not emitted (E306 subsumes)
- E304: `empty_match` (exhaustiveness.rs)
- E305: `non_exhaustive_match` (exhaustiveness.rs)
- E306: `unreachable_pattern` (exhaustiveness.rs)
- E307: `overlapping_range` (exhaustiveness.rs)
- E308: `irrefutable_while_let` (exhaustiveness.rs)
- E309: `irrefutable_guard_let` (exhaustiveness.rs)
- E310: `duplicate_match_binding` (match_pattern.rs)
- E311: `float_literal_in_pattern` (match_pattern.rs)
- E312: `unknown_enum_case` (match_pattern.rs)
- E313: `wrong_variant_arity` (match_pattern.rs)
- E314: `wrong_tuple_arity_in_pattern` (match_pattern.rs)
- E315: `or_pattern_inconsistent_bindings` (match_pattern.rs)
- E422: `disallowed_enum_conformance` (decl/conformance_rules.rs)
- E423: `conflicting_copyable_opt_out` (decl/conformance_rules.rs)
- E424: `negative_conformance_requires_language_feature` (decl/conformance_rules.rs)
- E430: `return_type_less_visible` (decl/visibility.rs)
- E431: `parameter_type_less_visible` (decl/visibility.rs)
- E432: `aliased_type_less_visible` (decl/visibility.rs)
- E433: `field_type_less_visible` (decl/visibility.rs)
- E447: `circular_type_alias` (compilation/type_alias_cycles.rs)
- E448: `type_alias_contains_infer` (compilation/type_alias_cycles.rs) — reserved; not emitted
- E449: `self_containing_struct` (compilation/struct_cycles.rs)
- E450: `circular_struct_containment` (compilation/struct_cycles.rs)
- E451: `circular_constraint` (compilation/constraint_cycles.rs)
- E459: `circular_protocol_inheritance` (compilation/protocol_cycles.rs)
- E461: `unknown_attribute` (compilation/unknown_attribute.rs)
- E700: `invalid_escape_sequence` (body/string_escape.rs)
- E701: `ascii_escape_out_of_range` (body/string_escape.rs)
- E702: `invalid_unicode_escape` (body/string_escape.rs)
- E703: `incomplete_escape_sequence` (body/string_escape.rs)

## Key Conventions

- Analyzers are **stateless ZSTs** — no fields, no mutable state
- Use `cx.query` to read ECS components (`NodeKind`, `Name`, `Callable`, `TypeAnnotation`, etc.)
- Use `cx.hir` to iterate the HIR body, `cx.typed` for resolved types
- Return `Vec<AnalyzeDiagnostic>` — the framework handles accumulation and memoization
- Use `DESCRIPTORS[N].id` and `DESCRIPTORS[N].default_severity` when constructing diagnostics
- Prefer early returns for inapplicable entities (wrong NodeKind, no return type, empty body, etc.)

## One analyzer per fact

If two analyzers ask the same question (e.g. irrefutable-pattern and
exhaustiveness both run Maranget), merge them. Two analyzers computing the
same thing drift — one gets updated, the other doesn't, diagnostics
disagree at the edges. Precedent: E302 / E303 / E306 all describe the
same pattern-matrix fact and live in `exhaustiveness.rs`. The old
`irrefutable_pattern.rs` was deleted when its logic duplicated the
exhaustiveness walk.

A single analyzer can own multiple diagnostic codes. Use a lookup helper
(`fn descriptor(id: &str) -> &'static DiagnosticDescriptor`) when
selecting by code at emit time.

## Pick one diagnostic per fact, and pick the one that labels the fix

When two codes describe the same situation (cause vs effect, umbrella vs
specific), emit only one. Prefer the diagnostic whose label points at the
code the user needs to change. Precedent: E306 (unreachable pattern —
labels the dead code) beats E303 (irrefutable-cause — labels the arm that
*caused* the dead code); they describe the same fact, E306 is actionable.

## Source-based dispatch for desugared constructs

When analyzing `HirExpr::Match`, branch on `source` first. `UserMatch`
gets the full diagnostic suite; desugared sources get source-specific
codes or are skipped entirely. See `MatchSource::is_desugared()` and the
per-source dispatch in `exhaustiveness.rs`.

## Errors-as-data on HIR nodes (when lowering must be the source of truth)

Some checks need information that only the lowering pass has — escape
decoding, integer parsing, etc. — and the resulting *value* must be
canonical (codegen consumes it; we can't decode twice). Don't push
diagnostic emission into lowering. Instead:

1. Define a typed error variant in `kestrel-hir` next to the literal /
   node it pertains to (e.g. `EscapeError` next to `HirLiteral::String`).
2. Have lowering return `(value, Vec<Error>)` purely — no `&mut sink`,
   no `ctx.accumulate`. Store the error list as a field on the HIR node
   itself, not as a side-table on `HirBody` (side-tables drift from the
   nodes they describe; see "Source-based dispatch" above for the same
   anti-pattern).
3. Write an analyzer that walks the relevant arena (`cx.hir.exprs`,
   `cx.hir.pats`) and translates the per-node error list into
   `AnalyzeDiagnostic`s.

Hash impls on the carrying enum should hash the *value*, not the error
list — errors are a derived property of the source and including them
breaks identity-based memo keys.

Precedent: `HirLiteral::String { value, escape_errors }` →
`body/string_escape.rs` (E700-E703). Decoder lives in
`kestrel-hir-lower/src/literal.rs`.

Do not check for desugared-ness via side-tables on `HirBody`
(`for_loop_matches` was removed for this reason). Use the enum on the
node.
