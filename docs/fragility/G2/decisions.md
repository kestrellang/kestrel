# G2 — decisions

## 1. The op list is generated from one macro, not written twice

**Decision:** `lib/kestrel-mir/src/op.rs` holds `op_ty_variants!`, an internal
macro whose entire body is the ten `Op` variants that carry a `TyId`.
`define_op_type_accessors!` expands it into both `op_type` and `op_type_mut`.
Three consumers — `mono::substitute_op_type`, `mono::collect_named_types`,
`mono::verify::verify_function` — go through those accessors.

Writing the list twice is not "a smell that led to" this bug; it *is* the bug.
`substitute_op_type` had all ten variants and `collect_named_types` had none,
and nothing anywhere connected them. A third site (`verify_function`) had
independently drifted to zero as well. Two hand-maintained match arms over the
same variant set will drift again — the only durable fix is that there is one
arm.

The `Option<&mut TyId>` return is deliberate over a visitor closure: it lets
`substitute_op_type` collapse to four lines with no borrow gymnastics, and it
composes with `if let` at both read sites.

**When you add an `Op` variant with a `TyId` payload, add it to
`op_ty_variants!` and nowhere else.**

## 2. The field half lives in the fixed-point loop, not in the collector

**Decision:** `collect_named_type_from_ty` was **not** taught to recurse into
struct fields. The discovery happens inside `resolve_types_and_layouts`'s loop,
at each field's existing resolution site.

The reason is a hard signature constraint, not taste. A generic struct's field
type is written in terms of the *definition's* type params (`Inner[U]`), so
seeding it requires `collect::substitute_and_resolve` with the instantiation's
`SubstMap` — and that takes `&mut TyArena`. `collect_named_types` takes
`&TyArena` and runs over `mono_bodies` before any substitution map exists. There
is no place in the collector where the concrete field type is available.

Inside the loop it already is: `substitute_and_resolve` is called one line above,
and its result is exactly what needs seeding. So the new call sits directly
after it, reusing `collect_named_type_from_ty` as the classifier — that function
is the single source of truth for "is this entity a struct or an enum", and
duplicating its `structs.contains_key` / `enums.contains_key` fork would have
been a second instance of the very defect being fixed.

### 2b. Why a staged `discovered` map rather than mutating in place

`for ((entity, type_args), kind) in &concrete_types` holds a shared borrow for
the whole body. That borrow is what made the old loop structurally incapable of
growing its own worklist. Staging into a per-pass local and draining after the
`for` is the minimal change that lifts the restriction without restructuring the
loop into index-based iteration (which would be subtly wrong: `IndexMap`
insertion during index iteration reads fine and is easy to get wrong later).

`discovered` is populated **unconditionally** — before, and regardless of,
whether `mono_size_and_align` resolves the field this pass. That ordering is the
whole point: a field whose layout is *not* yet known is precisely the one that
must be seeded, and gating the seed on resolution would recreate the bug.

### 2c. Cross-pass rediscovery is accepted, not deduped

`discovered` is fresh each pass, so an already-resolved field type is
re-collected every pass. This is harmless: the outer loop skips anything in
`layout_cache` immediately, and the drain only inserts keys not already in
`concrete_types`.

The alternative — threading a second "already seen" set through
`collect_named_type_from_ty`'s signature — would change a shared, widely-called
classifier to serve one caller's bookkeeping. Rejected.

### 2d. Termination

The break is `if !progress && !added_new`, where `added_new` is set only when a
key **not already in `concrete_types`** is inserted. Each pass therefore either
resolves a layout (at most once per key) or strictly grows `concrete_types`.

The key universe is finite for anything mono can compile at all. Unbounded
field-type growth needs polymorphic recursion (`struct N[T] { next: N[W[T]] }`),
which is impossible by value — the type would have infinite size — and behind a
`Pointer` it already diverges in *function* collection before reaching here. No
artificial depth cap was added: a cap that silently stops expanding would
reintroduce exactly the silent-drop failure mode this change exists to remove.

## 3. The guard was moved, not tuned — and the old one was kept

**Decision:** `check_type_concrete` gains a "`Named` with no `MonoStruct`/
`MonoEnum`" error. The pre-existing `layout.is_none()` loops in `verify_mono`
are left in place untouched.

The old checks are not wrong, they are *unreachable*: `MonoStruct` is inserted
inside `if all_resolved`, one line after its layout is assigned, so no
compiler-produced entry can have `layout: None`. Deleting them would be a
cosmetic change with a small chance of removing coverage for some future
construction path, so they stay as defense-in-depth. What they could never do is
notice a type that is *missing*.

`MirTy::Named` is safe to require a mono entry for because every primitive has
its own `MirTy` variant — verified against
`passes::layout::primitive_size_and_align`'s exhaustive match (`Bool`, `I8..I64`,
`F16..F64`, `Never`, `Str`, `Pointer`, `Ref`, `FuncThin`, `FuncThick`, `Error`).
A `Named` is always a user struct or enum.

### 3b. `verify_function`'s missing `Op1/Op2/Op3` arm was part of the same finding

Adding the guard without that arm would have left the op half half-covered: a
`SizeOf(GhostG[Int64])` naming an absent type would still slide through the
trailing `_ => {}`. Closing it also means the guard and the collector consult
the *same* ten-variant list, so neither can grow coverage the other lacks.

### 3c. Verified by running, per F40 §3b

`mono::verify::tests::named_type_absent_from_module_is_reported` hand-builds a
`MonoModule` whose body is `%1 = SizeOf(Ghost[]) %0` with no `MonoStruct` for
`Ghost`, asserts exactly one error, then inserts the `MonoStruct` and asserts the
module verifies clean. Unlike the test it replaces in spirit, the *first* half of
that pair is a shape mono really can emit.

The collector fix was verified the same way: temporarily disabling the new
`Op1|Op2|Op3` arm makes `op_type_operand_seeds_named_type` fail with
`left: None, right: Some(24)`, then passes again when restored. A guard or a
test that has not been observed failing on the bug it names is not coverage.

### 3d. The new guard fired nowhere

Full suite after the change: **3806 passed, 0 failed** (baseline 3802 + the four
new rows: two new `.ks` files × two backends). No pre-existing latent miss in the
stdlib or in testdata trips it. That is a genuine result, not an absence of
signal — the guard is a hard error in `CompilerDriver`, downstream of
`expand_destroy_copy`, and every test in the suite compiles through it.

## 4. Severity corrected: `medium` → `high`

Filed `medium` `single-source-of-truth`, which reads it as a duplication smell.
It is that, but the duplication produced **live silent wrong layouts on both
backends** with no diagnostic, and `Layout` is the argument type of
`Allocator.allocate` — an under-reported size is a heap under-allocation.
Silent wrong bytes at runtime is `high` regardless of how tidy the cause is.

## 5. Tests: the existing one is guard-shaped and cannot fail

`testdata/stdlib/memory/layout_of_non_copyable.ks` asserts `size == 8` on a
single-`Int64` **non-generic** struct. Both the correct answer and the buggy
fallback are 8, so it passes either way. The four `sizeof`/`alignof` intrinsic
tests are all `diagnostics`-kind — compile-only, they never look at a value.
Before this change **no test anywhere asserted a `sizeof`/`Layout` value for a
multi-word or generic-instantiated type.**

It was left alone rather than strengthened, so the record of what coverage
existed — and why it was insufficient — stays legible.

New coverage:

| test | what it pins |
| --- | --- |
| `testdata/stdlib/memory/layout_of_generic_via_sizeof_only.ks` | `GhostG[Int64]` = 24/8 reachable only via `sizeof`, against a non-generic control |
| `testdata/stdlib/memory/layout_of_nested_generic_field.ks` | `Outer[Int64] { i: Inner[Int64], x: Int64 }` = 32/8, against a flattened control |
| `mono::tests::op_type_operand_seeds_named_type` | all four of `SizeOf`/`AlignOf`/`PtrCast`/`PtrTo`, with the op result typed `i64` so the `Pointer[T]` correlation cannot carry it |
| `mono::tests::field_type_seeds_nested_generic` | the field half at the `monomorphize()` entry point, asserting **both** `Inner[i64]` (24) and `Outer[i64]` (32) — the container's disappearance was the worse half |
| `mono::verify::tests::named_type_absent_from_module_is_reported` | the guard, in both directions |

Both `.ks` files carry `// backends: cranelift,llvm`. Unlike F40 this is *not* a
cross-backend divergence — both backends were wrong identically, because the
wrong answer comes from a shared fallback in each one's `classify_named`. Running
both is still right: it pins that the fix reaches the codegen input, not just
mono's tables.

The Op half's proof is a Rust unit test rather than a `.ks` file on purpose. At
source level `lang.cast_ptr[_, T](p)` always yields a value typed
`lang.ptr[T]`, which the `Pointer` recursion seeds anyway — a `.ks` test of it
would pass with the fix reverted and prove nothing. Only a hand-built body can
break the correlation.
