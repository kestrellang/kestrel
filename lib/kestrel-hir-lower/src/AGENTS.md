# kestrel-hir-lower — design notes

## `Self` has no HirTy variant — known gap

HIR does not have a `HirTy::SelfType` variant. `Self` references get
name-resolved into whatever concrete entity is "nearest" in scope:

- Inside `extend Struct[T]` / `extend Enum[T]`: `Self` → `HirTy::Struct { entity }` or `HirTy::Enum { entity }` (the concrete nominal, with the extension's type args).
- Inside `extend Protocol`: `Self` → `HirTy::Protocol { entity }` — **the protocol entity itself**.
- Inside a protocol declaration's methods: same as above.
- Inside a function with `self: Self` in the receiver: handled separately in MIR lowering via `MirTy::SelfType` on the receiver slot only (see `function_lower.rs` / `body_lower.rs`).

The `extend Protocol` case leaks: bare `Item` inside `extend Iterator`
lowers to `AssocProjection { base: HirTy::Protocol(Iterator), assoc: Item }`.
In MIR that became `AssociatedProjection { base: Named(Iterator), protocol: Iterator, name: "Item" }` — a projection whose base is the protocol itself, not Self. Codegen's `substitute_type_with_self` had no hook to map `Named(Iterator)` back to the concrete self type, so the projection layout silently defaulted to `ptr` (8 bytes). Fine for `Item = Int64`/pointer-sized types (accidental-correct); wrong for sub-i64 items (`UInt8`, `Char` wrapping `UInt32`, `Grapheme`, …) — elements read back as garbage.

The current workaround lives in `kestrel-mir-lower/src/ty.rs` — when HIR has `AssocProjection { base: HirTy::Protocol(P), assoc } if assoc.parent == P`, MIR emits `Named(assoc_typealias, [])` instead of `AssociatedProjection`, so the existing `resolve_assoc_type_substs` walk in codegen resolves it via witness lookup. Codegen's `resolve_assoc_type_substs` also takes a `self_type` parameter now, with subst candidates tried first and `self_type` as fallback (critical: for methods like `Array.init[I](from: I)` the body's `I.Iter` must resolve via `I`, not `Self`).

**Principled fix** (not yet done): introduce `HirTy::SelfType` and have `build_self_hir_ty` emit it for the protocol case. Then `AssocProjection.base = HirTy::SelfType` → `MirTy::AssociatedProjection { base: MirTy::SelfType, … }` → `substitute_type_with_self` already handles it. The MIR-lowering workaround and the codegen `self_type`-fallback could both be removed.

## When adding a new HirTy variant

If you add `HirTy::SelfType` (see above), update:

- `kestrel-mir-lower/src/ty.rs::lower_type` — produce `MirTy::SelfType`.
- `kestrel-hir-lower/src/ty.rs::build_self_hir_ty` — emit `HirTy::SelfType` for protocol Self.
- Every analyzer that walks `HirTy` — search for `HirTy::` match sites and handle the new variant (expect name-res, type-check, conformance).
- Remove the workaround emit in `kestrel-mir-lower/src/ty.rs` (the `AssocProjection { base: HirTy::Protocol(P), assoc } if *entity == protocol` branch).
- Re-simplify `resolve_assoc_type_substs` in `kestrel-codegen-cranelift/src/function.rs` if the self_type fallback is no longer needed.
