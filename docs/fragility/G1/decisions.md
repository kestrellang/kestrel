# G1 — decisions

## 1. Fix the ORDERING, not the predicate

**Decision: run `drop_fix::fix_drop_behaviors` between `lower_types` and
`lower_functions` in `lower_items`.** One line, no new state, no new query.

The tempting cheap fix was to widen the fallback disjunct in
`setup_init_field_flags` from `is_non_copyable(field_ty)` to
`copy_behavior(field_ty) != CopyBehavior::Bitwise`, so a `Cloneable` wrapper
would at least be treated as droppable.

**Rejected: it is provably insufficient, and it was compiled to prove it.** A
`deinit` does not affect copy semantics, so

```kestrel
struct Res  { var id: Int64; deinit { … } }   // no conformance clause
struct Wrap { var r: Res }
```

leaves both types `CopyBehavior::Bitwise` while `Wrap` is droppable purely
through `fix_drop_behaviors`. That program leaks today and would still leak
under the widened predicate. It now ships as
`memory_model/deinit/init_field_reassign_default_copyable_field_drop.ks` and its
`init?` twin, specifically so a future "simplification" back to a copy-based
predicate fails loudly.

More fundamentally, the widening treats a *copy* fact as a proxy for a *drop*
fact. The two are independent in this language (`Copyable` + `deinit` is legal
and tested — see `deinit_copyable_type_allowed.ks`). Any proxy is a second
source of truth that drifts. The ordering fix makes the primary disjunct,
`needs_drop`, simply correct.

## 2. Verified that pass 1 fully populates the pass's inputs

`fix_drop_behaviors` reads `module.structs[e].fields[i].ty`,
`module.structs[e].type_info.drop`, and the `cases[v].payload_fields` /
`type_info.drop` of `module.enums`, plus `module.ty_arena` through `needs_drop`.

- `struct_lower::lower_struct` resolves every stored-instance field to a
  concrete `TyId` and pushes it into `def.fields` **before** `ctx.module.add_struct(def)`,
  and sets `type_info` in the same builder. `enum_lower::lower_enum` does the
  same for payload fields. Nothing is back-filled later.
- The only non-test `add_struct` / `add_enum` call sites in the compiler are
  `struct_lower.rs`, `enum_lower.rs`, `body/closure.rs` (closure-environment
  structs), and `passes/clone_shim.rs`. The last two are the exception handled
  in decision 3; everything else is `lower_types`.

## 3. KEEP the existing `passes/mod.rs` call — the two are not duplicates

**Decision: both call sites stay, each with a comment naming the other.**

Body lowering *synthesizes* types after the hoisted call has run:
`body/closure.rs` builds a closure-environment struct and `add_struct`s it
mid-pass-2. The hoisted call cannot see those. The `Stage::DropFix` call still
has to promote them (and any type `clone_shim` later adds). Collapsing the two
would silently reintroduce G1 for one half or leave closure environments
undropped for the other, so both sites now carry a comment explaining why the
overlap is deliberate.

## 4. Idempotence is load-bearing, and was verified by reading the pass

Running the pass twice is only safe if the second run is a no-op on an
unchanged module. `passes/drop_fix.rs` was read in full:

- `fix_structs` computes `droppable_fields` from the current module; if empty it
  `continue`s **before** touching anything.
- The only mutations are `DropBehavior::None → StructDrop { deinit: None, fields }`
  and, on an existing `StructDrop`, `fields.push(f)` guarded by
  `if !fields.contains(&f)`. `fix_enums` is the same shape, matching variants by
  `VariantIdx` and pushing a missing variant or missing field indices.
- No branch removes a field, clears `drop`, or overwrites `deinit`; the `_` arm
  (a struct carrying `EnumDrop`, or vice versa) does nothing. `DropBehavior` has
  only three variants, so those are the only cases.

The pass is therefore **monotone and additive**, and its own outer `loop`
already re-runs it to a no-change fixed point by design. A second invocation
over a module already at that fixed point recomputes the identical
`droppable_fields`, finds every index present, reports `changed == false`, and
exits after one sweep. Empirically, the full 3795-test suite is clean — a
non-idempotent promotion would duplicate field indices in `StructDrop.fields`
and double-free through the synthesized drop shims across the whole stdlib.

## 5. The doc comment is part of the fix

The stale comment on `lower_items` is what made this invisible for as long as it
was: it asserted the exact invariant that was being violated, so nobody reading
`setup_init_field_flags` had reason to check. It now states plainly that
`CopyBehavior` **is** final after pass 1 and `DropBehavior` is **not**, names
`fix_drop_behaviors` as the thing that finalizes it, and records that the pass
runs twice on purpose.

The `is_non_copyable` OR-fallback in `body/mod.rs` was kept (defense in depth)
but its comment was corrected — it previously read as though it covered the
ordering hazard, which it never did.

---

# Findings to file (out of scope for G1)

## `verify_ossa`'s four `addr_*` checks are inert for EVERY initializer body

`verify.rs` has exactly the rule that would have caught G1's own reassignment
shape:

```
addr_store_init  →  "store_init on field {..} but field already init"
```

It never fires. `AddrKind::SubField` state is created **only** by
`InstKind::Uninit`. An init body's `self` is a `@mut_borrow` **parameter**, never
`Uninit`, so it has no `state.addrs` entry — and all four `addr_*` checks
(`addr_store_init`, `addr_require_init`, and the two beside them) silently no-op
for every initializer in the language.

This was left alone deliberately. Seeding `state.addrs` from the init `self`
parameter would activate `addr_require_init` across all 253 init-bearing test
files simultaneously — a completely different blast radius from a one-line
ordering fix, and one that would bury the G1 result. It should be filed
separately and **first run in detection-only mode** (count the violations
without failing) to size the impact before any enforcement lands.
