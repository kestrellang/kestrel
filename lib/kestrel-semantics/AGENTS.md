# kestrel-semantics

Shared semantic queries: conformance, copy semantics, `Static`, `Sized`,
existential dispatch, recursive enums. See `docs/architecture.md`.

- **Query-only.** This crate holds the facts several stages need, so no stage
  reinterprets conformance syntax on its own. Add a fact here when a second
  stage needs it.
- **Pick the right conformance tier**: `ResolvedConformances` for what one
  entity wrote (with negatives and spans), `ConformingProtocols` for the
  transitive set, `DeclaresConformanceTo` for membership. None of them checks a
  conditional `where`; that is `type_satisfies` in the conformance checker.
- **Copy, `Static` and `Sized` rules aren't written here.** The decision trees
  are in kestrel-copy-fold and kestrel-structural; this crate implements their
  layer traits over `HirTy` (`HirCopyLayer`, `HirStaticWalk`, `HirSizedWalk`).
- **Recursive types use `CycleMode::Fixpoint`** (`NominalCopySemantics` starts
  at `Copyable` and descends). Don't add an in-progress stack.
- **Only stored instance fields and case payloads count.** Computed properties
  and `static` fields store nothing in the instance.
- **`recursive_enum.rs` is the single answer** to "which cases reach their own
  enum by value", for both E429 and `indirect` boxing in MIR.
- Builtin protocols are found with `ResolveBuiltin` only. There is no name
  fallback.
