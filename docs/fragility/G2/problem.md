# G2 — mono's type collector and its substituter are supposed to be mirrors, and aren't

`high` (audit filed it `medium`; corrected — see `decisions.md` §4) ·
`single-source-of-truth` · crate: `kestrel-mir`

## Symptom

A type that mono never *seeds* gets no `MonoStruct`/`MonoEnum`, so both backends'
`classify_named` fall through to `else { Scalar(ptr_scalar) }`. `sizeof` answers
**8** for a 24-byte struct; a nested aggregate answers **8** for 32 bytes.
`struct_field_offset` returns a bare `0` and `struct_field_type` hands back the
*container* type as the field type. No diagnostic, no ICE, both backends agree —
because they share the same fallback.

Measured, `kestrel build`, `KESTREL_BACKEND` ∈ {cranelift, llvm}, before the fix:

| program | cranelift | llvm | expected |
| --- | --- | --- | --- |
| `GhostG[Int64]` (3×`Int64`) reachable only via `Op::SizeOf` | `8 / 8` | `8 / 8` | `24 / 8` |
| `Outer[Int64] { i: Inner[Int64], x: Int64 }` | `8 / 8` | `8 / 8` | `32 / 8` |
| `Flat[Int64]` (same 4 words, no nested field — control) | `32 / 8` | `32 / 8` | `32 / 8` |
| `GhostBig` (non-generic, 3×`Int64` — control) | `24 / 8` | `24 / 8` | `24 / 8` |

The two controls isolate the loss precisely: flattening the nested field, or
dropping the generic parameter, restores the right answer.

**A plain non-generic struct does not reproduce this.** Concrete functions are
unconditionally mono roots, so a non-generic type's `init` seeds it as a value
type no matter what. The type has to be a *generic instantiation* whose `init`
is never monomorphized.

### Not cosmetic

`Layout` is the allocator ABI — `lang/std/memory/allocator.ks:30`,
`allocate(layout: Layout)`. An under-reported `size` is a heap
under-allocation, and every subsequent write past byte 8 is out of bounds.
Codegen independently emits an 8-byte slot for what should be a 32-byte
by-value parameter.

## Root cause — two independent halves, both live

`collect_named_types` (`lib/kestrel-mir/src/mono/mod.rs`) walks:

* value types,
* block-param types,
* `InstKind::Struct { ty }` / `Enum { enum_ty }` / `Array { element_ty }`,
* `InstKind::Literal` immediates `SizeOf` / `AlignOf` / `NullPtr`.

### Half 1 — zero `Op` type operands

Ten `Op` variants carry a `TyId`: `PtrFromAddress`, `PtrRead`, `PtrWrite`,
`PtrNull`, `PtrTo`, `PtrCast`, `PtrBitcast`, `SizeOf`, `AlignOf`, `StackAlloc`.
`substitute_op_type` handled **all ten**. `collect_named_types` handled
**zero**. The two functions are meant to be mirrors — one substitutes the type
operand, the other collects it — and each had its own hand-written match arm.

Only `SizeOf`/`AlignOf` genuinely escape today: the other eight produce a
`Pointer[T]` or a `T` result value, and `collect_named_type_from_ty`'s `Pointer`
recursion re-seeds `T` from the *value's* type. **That correlation is incidental,
not enforced.** Nothing makes a `PtrCast(T)` result be typed `Pointer[T]`; the
moment one isn't, the same silent fallback applies. The unit test
`mono::tests::op_type_operand_seeds_named_type` breaks the correlation on
purpose — it types the op result `i64` — so the coverage does not depend on it.

### Half 2 — struct/enum field types, independently live and worse

`collect_named_type_from_ty` recurses into type args, `Pointer`, `Ref` and
`Tuple`. It never descends into a struct's **field** types. So `Inner[Int64]`,
reachable only as a field of `Outer[Int64]`, is never in the worklist.

The fixed-point loop that computes layouts could not fix this up either: it
iterates `&concrete_types` by **shared borrow**, so it was *structurally* unable
to add a type it discovered while resolving fields. When a field type is
unseeded, `mono_size_and_align` returns `None`, `all_resolved` goes false — and
because `MonoStruct` is only inserted inside `if all_resolved`, the
**containing** struct is dropped from `mono_structs` too. Two types vanish, one
silently causing the other.

## Why `verify_mono`'s guard was inert — the same class as F40's

`mono/verify.rs` reported entries whose `layout.is_none()`:

```rust
for s in module.structs.values() {
    if s.type_info.layout.is_none() { errors.push(…"missing layout") }
}
```

But a `MonoStruct` is only ever inserted inside `if all_resolved`, on the line
*after* `ms.type_info.layout = Some(…)`. **Every entry has a layout by
construction.** The guard checks *presence with a null field*; the failure mode
is *absence from the map*. Its own test had to hand-build a `MonoModule` with a
`None` layout to make it fire — a shape the compiler cannot produce.

`verify.rs` also carried its **own independent copy of the op-half gap**:
`verify_function` had no `InstKind::Op1/Op2/Op3` arm at all, so op type operands
fell through the trailing `_ => {}` and were never checked for concreteness
either. Three places needed the same ten-variant list; three places had three
different answers (all ten / none / none).

## The fix

1. **One canonical enumeration, macro-backed** (`lib/kestrel-mir/src/op.rs`).
   `op_ty_variants!` names the ten variants exactly once;
   `define_op_type_accessors!` generates `op_type(&Op) -> Option<TyId>` and
   `op_type_mut(&mut Op) -> Option<&mut TyId>` from it. `substitute_op_type`,
   `collect_named_types` and `verify_function` all consume those. Two
   hand-written match arms is how this bug happened; there is now one list.
2. **`collect_named_types` gains an `Op1|Op2|Op3` arm** via `op_type`.
3. **The field half goes in the fixed-point loop, not the collector.** Field
   types are generic and need `collect::substitute_and_resolve`, which needs
   `&mut TyArena` — `collect_named_types` only has `&TyArena`. A per-pass local
   `discovered` map is populated **unconditionally** at each field's resolution
   site (struct fields and enum case payload fields), by calling the existing
   recursive `collect_named_type_from_ty` — reusing that classifier as the
   single source of truth for "is this a struct or an enum". After the `for`
   ends — legal only outside the shared borrow — `discovered` is drained into
   `concrete_types`, and the break becomes `if !progress && !added_new`.
4. **The guard moves to where it can fire.** `check_type_concrete` now reports a
   `MirTy::Named` that is in neither `module.structs` nor `module.enums`. A
   `Named` is always a genuine user struct/enum — every primitive has its own
   `MirTy` variant, confirmed against `passes::layout::primitive_size_and_align`'s
   exhaustive match — so absence means codegen will silently mis-size the value.
   The old `layout.is_none()` checks stay as harmless defense-in-depth.

After the fix all four table rows above read `24 / 8` and `32 / 8` on both
backends.
