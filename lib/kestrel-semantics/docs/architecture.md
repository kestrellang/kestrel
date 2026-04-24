# kestrel-semantics Architecture

Shared semantic queries for conformance facts and copy semantics.

## Pipeline Position

Source Text -> Tokens -> CST -> AST Build -> Name Res -> HIR Lower -> Semantics -> Analyze/Infer/MIR
                                                                    ^^^^^^^^^
                                                                    this crate

## Core Types

| Type | Module | Description |
|------|--------|-------------|
| `ResolvedConformances` | `lib.rs` | Resolves direct positive/negative conformance entries to protocol entities when possible |
| `ProtocolRefines` | `lib.rs` | Checks whether one protocol is or inherits another protocol |
| `NominalCopySemantics` | `lib.rs` | Computes `Copyable`, `Cloneable`, or `NotCopyable` for a nominal entity |
| `TypeParamCopyRequirement` | `lib.rs` | Interprets default, `Cloneable`, and `not Copyable` generic requirements |

## Module Map

| File | Responsibility |
|------|----------------|
| `src/lib.rs` | Query keys, data types, and helper functions |

## Dependencies

| Crate | Usage |
|-------|-------|
| `kestrel-ast-builder` | Declaration components and raw conformance items |
| `kestrel-name-res` | Resolving protocol names and builtin entities |
| `kestrel-hir-lower` | Lowering field/payload type annotations for copy semantics |
| `kestrel-hir` | Builtin metadata and HIR type shape |
