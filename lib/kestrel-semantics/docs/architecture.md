# kestrel-semantics Architecture

Shared semantic queries for conformance facts and copy semantics. Intentionally query-only: it centralizes facts that analyzers, type inference, move tracking, and MIR lowering all need, so no phase reinterprets raw conformance syntax on its own.

## Pipeline Position

Source Text -> Tokens -> CST -> AST Build -> Name Res -> HIR Lower -> Semantics -> Analyze/Infer/MIR
                                                                    ^^^^^^^^^
                                                                    this crate

## Core Types

| Type | Module | Description |
|------|--------|-------------|
| `ResolvedConformanceSet` / `ResolvedConformance` | `lib.rs` | Direct conformance entries with polarity, resolved target (`Protocol`/`NonProtocol`/`Unresolved`), AST type, and span |
| `ConformancePolarity` | `lib.rs` | `Positive` (`: P`) or `Negative` (`: not P`) |
| `CopySemantics` / `CopySemanticsInfo` | `lib.rs` | `Copyable`/`Cloneable`/`NotCopyable`, plus the reason (explicit negation, non-copyable child, explicit `Cloneable`, default) |
| `CopyRequirement` | `lib.rs` | What a generic context demands of a type param: `RequiresCopyable`/`RequiresCloneable`/`MayBeNonCopyable` |

## Queries

| Query | Output | Description |
|-------|--------|-------------|
| `ResolvedConformances` | `ResolvedConformanceSet` | The conformance entries written on *one* entity (declaration or extension), each with polarity and resolved protocol target |
| `ProtocolRefines` | `bool` | Whether one protocol is or transitively refines another (reflexive) |
| `ExplicitlyNegatesProtocol` | `bool` | Whether an entity writes `: not P` for the given protocol |
| `DeclaresConformanceTo` | `bool` | Whether an entity *declares* conformance to a protocol by any route — direct declaration, `extend`, or refinement (membership in `ConformingProtocols`). "Declares", not *satisfies*: it never evaluates conditional `where` clauses |
| `IsBuiltinProtocol` | `bool` | Whether a protocol entity is a specific builtin (`Copyable`, `Cloneable`, ...) |
| `TypeParamCopyRequirement` | `CopyRequirement` | Folds the where clauses visible from a param's declaration and use contexts: default `RequiresCopyable`, a `Cloneable`-refining bound gives `RequiresCloneable`, `T: not Copyable` gives `MayBeNonCopyable` |
| `NominalCopySemantics` | `CopySemanticsInfo` | Entity-level classification of a struct/enum, with the reason |
| `ConditionalCopyableParams` | `Vec<usize>` | For a `not Copyable` type with `extend X[...]: Copyable where ...`, the type-arg positions whose copyability gates the instantiation's; empty when the type isn't conditionally copyable |

## Functions

| Function | Description |
|----------|-------------|
| `protocol_allows_negative_conformance` | Whether `: not P` is permitted on a protocol — true only for builtin language-feature protocols with implicit conformance (e.g. `Copyable`). A plain function, not a query: the body is one `EntityBuiltin` lookup, cheaper to recompute than a memo slot |
| `hir_type_copy_semantics` | Copy semantics of any `HirTy`: folds tuples, routes params through `TypeParamCopyRequirement`, and nominal instances through per-instantiation folding (see below) |
| `hir_type_conforms_to_protocol` | Protocol check for a `HirTy`: routes `Copyable`/`Cloneable` through copy semantics, everything else through `DeclaresConformanceTo` (nominals) or where-clause bounds (params) |

## Conformance: Three Tiers

Pick the tier matching the question being asked:

1. **Raw + polarity** — `ResolvedConformances`: what was literally written on one entity, positives *and* negatives, with spans. For diagnostics about declarations.
2. **Transitive set** — `ConformingProtocols` (kestrel-name-res): the closure of positive declarations across the type, its extensions, and protocol refinement.
3. **Membership** — `DeclaresConformanceTo`: "is P in that closure?". Still declares-only; *satisfaction* of a conditional `where`-gated conformance is the bound-aware `type_satisfies` check downstream in the conformance checker.

## Copy-Semantics Layering

Copyability is per-instantiation; the layers build bottom-up:

```
TypeParamCopyRequirement      what does the context demand of a bare param?
NominalCopySemantics          entity-level class: explicit `: not Copyable`
                              > non-copyable stored child > declared Cloneable > Copyable
ConditionalCopyableParams     which arg positions gate a `not Copyable` type's
                              `extend ...: Copyable where ...` conformance
hir_type_copy_semantics       instance-level fold over the gating args:
                              any NotCopyable -> NotCopyable,
                              else any Cloneable -> Cloneable, else Copyable
```

`ConditionalCopyableParams` is the single source of truth for gating positions: this crate, the inference solver (`nominal_conforms_copyable`), and MIR `copy_behavior` must all agree on each instantiation. Only stored fields and enum payloads contribute to the child scan; computed properties never do.

## Key Design Decisions

- **Cycle guard**: `NominalCopySemantics` on a recursive type would re-enter itself, and the query framework panics on re-entry. A thread-local in-progress stack (side-channel state, invisible to dependency tracking) lets callers detect the cycle first and fall back to `Copyable` on self-reference.
- **`not Copyable` name fallback**: when the `Copyable` builtin entity isn't registered (stdlib-less test fixtures), a last-segment string match keeps `struct H: not Copyable {}` non-copyable. `ResolveBuiltin` is the source of truth — don't extend the string-match pattern.

## Module Map

| File | Responsibility |
|------|----------------|
| `src/lib.rs` | Query keys, data types, and helper functions |

## Dependencies

| Crate | Usage |
|-------|-------|
| `kestrel-ast-builder` | Declaration components and raw conformance items |
| `kestrel-name-res` | Resolving protocol names, builtin entities, `ConformingProtocols` |
| `kestrel-hir-lower` | Lowering field/payload annotations and extension target args |
| `kestrel-hir` | Builtin metadata and HIR type shape |
