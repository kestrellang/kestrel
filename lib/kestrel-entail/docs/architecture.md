# kestrel-entail Architecture

The one where-clause entailment kernel. It answers "does `T` satisfy `P`?" and
"do this extension's `where` clauses hold for receiver `R`?" for every layer
that asks.

## Pipeline Position

```
Source Text → Tokens → CST → AST Build → Name Res → HIR Lower → Type Infer → MIR → Mono → Codegen
                                                                ^^^^^^^^^^           ^^^^
                                                    solver + analyzer (FrontEndLayer)  MonoLayer
```

A dependency-graph leaf (only `kestrel-hecs`), so mono can use it without
pulling in the front end.

## Core Types

| Type | Description |
|------|-------------|
| `Tri` | Three-valued answer: `Holds`, `Fails`, `Unknown`. `Unknown` means an open inference variable (or a depth cut-off) blocks the answer. It is never "permit" |
| `ETy` | The kernel's small type language: `Nominal`, `Ref`, `Tuple`, `Never`, `Param`, inference `Var`, `Poison`, … |
| `Goal` | `Conforms { ty, protocol }` or `Equal(a, b)` (equality clauses such as `where T.Item = Int64`) |
| `EntailLayer` | The trait each caller implements to supply world facts: which declarations supply a conformance (`supply`), associated-type bindings (`assoc_binding`), structural questions |
| `Supply` / `Source` | A declaration that supplies a conformance, with its where-clause goals |

## Entry Points

| Function | Use |
|----------|-----|
| `entails(layer, goal)` | Prove one goal |
| `extension_applies(layer, …)` | Do an extension's clauses hold for a receiver? |
| `more_specific` / `more_specific_rule` | Pick the more specific of two applicable extensions |
| `subst` | Substitute bound params into an `ETy` |

## Layers

| Layer | Crate | Converts |
|-------|-------|----------|
| `FrontEndLayer` | `kestrel-type-infer/src/entail_layer.rs` | `HirTy` for the analyzer (`conformance::type_satisfies`) |
| `SolverLayer` | `kestrel-type-infer/src/solver_entail.rs` | `TyVar`; an open var becomes `ETy::Var`, so questions about it are `Unknown` |
| `MonoLayer` | `kestrel-mir/src/mono/entail.rs` | Concrete `TyId`s after instantiation |

## Key Design Decisions

- **One kernel, N data sources.** There used to be five evaluators with three
  different answers to "unknown", and equality clauses were skipped as
  satisfied. Matching, substitution, the three-valued AND/OR and specificity
  now live only here. Don't write a second evaluator in a layer.
- **Unknown is explicit.** The solver re-queues an `Unknown` goal. Callers whose
  types are always complete (analyze, mono) decide what `Unknown` means for
  them.
- **Poison holds.** `ETy::Poison` (an already-reported error) answers `Holds`,
  so no cascade is reported on top of the first diagnostic.

## Module Map

| File | Responsibility |
|------|---------------|
| `lib.rs` | The kernel: types, `EntailLayer`, `entails`, specificity, substitution |
| `tests.rs` | Kernel tests over a toy world (`Box[T]: Show where T: Show`) |

## Dependencies

| Crate | Usage |
|-------|-------|
| `kestrel-hecs` | `Entity` |
