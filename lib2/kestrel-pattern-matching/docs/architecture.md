# kestrel-pattern-matching — Architecture

Pattern matching analysis and compilation for the Kestrel compiler. Checks match expressions for exhaustiveness, detects unreachable arms and overlapping ranges, and compiles patterns into decision trees for codegen.

## Pipeline Position

```
Source → Lex → Parse → AST Build → Name Res → HIR Lower → Type Infer → Analyze / Codegen
                                                                            ^^^
                                                                     this crate (shared)
```

Two consumers depend on this crate:

- **`kestrel-analyze`** — runs exhaustiveness/redundancy checks, emits KS304–KS307 diagnostics
- **Execution graph lowering** — compiles patterns to decision trees for codegen

## Data Flow

```
HirPat ──► flatten() ──► FlatPat ──► PatternMatrix ──┬── is_useful() ──► ExhaustivenessResult
                                                      │
                                                      └── compile() ──► DecisionTree
```

1. `flatten()` converts arena-based `HirPat` into value-based `FlatPat`, resolving implicit variants and expanding struct fields
2. `FlatPat` rows are assembled into a `PatternMatrix` with column types from `ResolvedTy`
3. The Maranget algorithm checks usefulness (exhaustiveness + redundancy)
4. Decision tree compilation produces an IR for codegen

## Core Types

| Type | Module | Description |
|------|--------|-------------|
| `Constructor` | `constructor.rs` | Head of a pattern: `True`, `False`, `Variant`, `Tuple`, `Struct`, `IntLiteral`, `IntRange`, etc. |
| `TypeShape` | `constructor.rs` | Classifies `ResolvedTy` → constructor space (`Bool`, `Enum`, `Tuple`, `Struct`, `Unit`, `Never`, `Infinite`) |
| `FlatPat` | `flat_pat.rs` | Normalized pattern: `Wildcard`, `Ctor { ctor, children }`, `Or(alternatives)` |
| `PatternMatrix` | `matrix.rs` | Matrix of `FlatPat` rows with `ResolvedTy` column types |
| `PatternRow` | `matrix.rs` | One row: patterns + arm index + has_guard flag |
| `ExhaustivenessResult` | `usefulness.rs` | Output: is_exhaustive, missing_patterns, redundant_arms, overlapping_arms |
| `UsefulnessResult` | `usefulness.rs` | Single usefulness check: is_useful + optional witness |
| `Witness` | `witness.rs` | Example uncovered value for error messages (`.None`, `(_, 42)`) |
| `DecisionTree` | `decision_tree.rs` | Codegen IR: `Switch`, `Success`, `Guard`, `Failure` |
| `Binding` | `decision_tree.rs` | Variable binding with `LocalId`, name, type, and access path |

## Module Map

| File | Responsibility |
|------|---------------|
| `lib.rs` | Public API: `check_match()`, `is_irrefutable()`, `compile_decision_tree()` |
| `constructor.rs` | Constructor enum, TypeShape, type→constructor queries, field type resolution |
| `flat_pat.rs` | FlatPat enum, `flatten()` (HirPat→FlatPat), `decompose()` (pattern decomposition) |
| `matrix.rs` | PatternMatrix, `specialize()` (S(c,P)), `default_matrix()` (D(P)), multi-column support |
| `usefulness.rs` | Maranget algorithm, `check_match()`, redundancy detection, overlap detection |
| `witness.rs` | Witness enum, Display impl for error messages |
| `decision_tree.rs` | Decision tree compilation, column selection heuristic, binding extraction |

## Algorithm

Based on Luc Maranget's papers:

- "Warnings for pattern matching" (JFP 2007) — exhaustiveness and redundancy
- "Compiling Pattern Matching to Good Decision Trees" (2008) — decision tree compilation

### Usefulness

A pattern `q` is **useful** w.r.t. a matrix `P` if some value matches `q` but no row in `P`.

```
is_useful(P, q):
  if P is empty → useful (nothing blocks q)
  if P has 0 columns → useful iff no unguarded row exists
  if q[0] is constructor c → specialize(P, c), specialize(q, c), recurse
  if q[0] is wildcard:
    for each constructor c of the column type:
      if c is uncovered → useful (witness: c)
      else → specialize and recurse
    if infinite type → check default matrix
```

- **Exhaustiveness** = wildcard `_` is NOT useful
- **Redundancy** = arm is NOT useful against prior arms
- **Guards** don't cover: guarded arms are excluded from the matrix

### Constructor Space (TypeShape)

| Kestrel Type | TypeShape | Constructors | Exhaustive? |
|-------------|-----------|--------------|-------------|
| `Bool` | `Bool` | `True`, `False` | Yes (2) |
| `enum E { A, B(T) }` | `Enum` | `Variant(A, 0)`, `Variant(B, 1)` | Yes (N) |
| `(T, U)` | `Tuple(2)` | `Tuple(2)` | Yes (1) |
| `struct S { x, y }` | `Struct` | `Struct(S, 2)` | Yes (1) |
| `()` | `Unit` | `Unit` | Yes (1) |
| `Never` | `Never` | (none) | Yes (0) |
| `Int64`, `String` | `Infinite` | unbounded | No |
| `Array[T]` | `Infinite` | variable length | No |

To make a new type exhaustively matchable, add a variant to `TypeShape` and a match arm in `TypeShape::classify()`.

### Decision Tree Compilation

```
match x {
    .None => 0
    .Some(0) => 1
    .Some(n) => n
}
```

Compiles to:

```
Switch(x, [
    None → Success(arm 0, []),
    Some → Switch(x.Some.0, [
        0 → Success(arm 1, []),
        _ → Success(arm 2, [n = x.Some.0])
    ])
])
```

Column selection uses the **necessity heuristic**: pick the column with the most distinct constructors to minimize branching.

## Deduplication Invariants

Each piece of pattern-matching logic exists in exactly one place:

| Logic | Location | Used by |
|-------|----------|---------|
| Pattern decomposition | `FlatPat::decompose()` | matrix specialize, decision tree compile |
| Constructor field types | `Constructor::field_types()` | matrix specialize, decision tree compile |
| Constructor compatibility | `Constructor::matches()` | FlatPat decompose, matrix specialize |
| Type classification | `TypeShape::classify()` | usefulness, irrefutability |
| Or-pattern expansion | `decompose_all()` in matrix | matrix specialize |

## Dependencies

```
kestrel-pattern-matching
├── kestrel-hecs          (Entity, QueryContext)
├── kestrel-ast           (AstType for field/param type resolution)
├── kestrel-ast-builder   (NodeKind, Callable, Name, TypeParams, TypeAnnotation)
├── kestrel-hir           (HirBody, HirPat, HirMatchArm)
├── kestrel-type-infer    (ResolvedTy, TypedBody)
└── kestrel-span2         (Span)
```
