# G16 — decisions

## 0. The decision: G16 is deliberately NOT fixed yet

**G16 stays open and unfixed, and `docs/fragility-audit.md` should keep its
unchecked box.**

Fixing G16 in isolation converts **six false errors into six silent
miscompiles**. The false E101 on abstract positions is, right now, the only
thing preventing generic code from reaching a `branch` instruction that reads
a struct's raw bytes instead of calling its `boolValue()` witness (see
`problem.md` §3, re-verified by running at `6d0e30b1`). A "fix" that only
relaxes the analyzer gate makes `if x` in a generic body compile and then do
the wrong thing at runtime with no diagnostic.

**The prerequisite, stated plainly:**

> MIR's `lower_if` (`lib/kestrel-mir-lower/src/body/control.rs:20-52`) must
> call `boolValue()` on a condition whose type is not `lang.i1`, and branch on
> that result, **before** the E101 gate in `condition_check.rs` can be
> relaxed for any abstract position.

Until that lands, the false positive is load-bearing. It is the wrong
diagnostic for the right reason.

---

## 1. Fix options considered, and why none is landable today

### (a) Add permit arms to `condition_check.rs` (~15 lines)

Give `conforms_to_protocol` and `is_bool` arms for `Param`, `SelfType`,
`AssocProjection`, `Opaque`, `Ref`.

**Rejected, twice over.**

- Blanket-permitting the abstract variants also permits `l_nobound.ks`
  (`func f[T](flag: T)` with **no** bound), which E101s correctly today. It
  trades a false-positive class for a false-negative class.
- Even a bound-checking version lets generic code reach §3's garbage branch.

### (b-i) Route through `kestrel_type_infer::type_satisfies`

Dependency-legal — `kestrel-analyze` already calls it from
`compilation/entry_point.rs:226` and `compilation/conformance_completeness.rs:1648`
— and the `ResolvedTy → HirTy` bridge already exists as the private
`resolved_ty_to_hir` (`conformance_completeness.rs:1604-1620`).

**Rejected: it answers the wrong question.** That helper collapses every
abstract variant to `HirTy::Infer` via `_ => HirTy::Infer(sp)`, and
`type_satisfies` permits `Infer` unconditionally (`conformance.rs:125-129`).
So the composite predicate is "can I *disprove* this?", not "is the bound
*satisfied*?". It would permit all six false positives **and** the
`l_nobound.ks` true positive alike.

*Aside worth fixing whenever that file is next touched:* `resolved_ty_to_hir`'s
doc comment claims "Concrete nominals/tuples/**refs** are reproduced
faithfully". Verified: there is no `ResolvedTy::Ref` arm — refs fall to
`_ => HirTy::Infer`. The comment is wrong.

### (b-ii) Route through `TypeResolver::conforms_to`

`kestrel-type-infer/src/resolve.rs:504-640` is the semantically right
predicate. Verified by reading: its `Param` arm goes through
`collect_param_protocol_bounds`; it has real `SelfType`, `TypeAlias`,
`AssocProjection` and `Opaque` arms; it special-cases the structural builtins
(`Copyable`/`Cloneable`/`Static`) that `ConformingProtocols` cannot answer.
It would fix the abstract-position cases **while keeping** the `l_nobound.ks`
rejection, because a param with no bound has no `BooleanConditional` in
`collect_param_protocol_bounds`.

**Mechanically blocked.** It takes `&TyKind`, and `TyKind`'s abstract variants
carry `TyVar`:

```rust
pub struct TyVar(pub(crate) u32);                       // ty.rs:10
TyKind::AssocProjection { base: TyVar, assoc: Entity }  // ty.rs:80
```

`TyVar` is not constructible outside `kestrel-type-infer`, so
`TyKind::AssocProjection` cannot be built from `kestrel-analyze` at all, and
no `ResolvedTy → TyKind` conversion exists.

**Correction to the earlier read: (b-ii) would fix four of the six, not all
six.** The two `&` shapes (`i_ref.ks`, `j_ref_bool.ks`) would still be
rejected. `resolve.rs:614-625`'s `Ref` arm answers via the synthetic `lang.&`
entity's `ConformingProtocols`, i.e. it needs an
`extend &T: BooleanConditional where T: BooleanConditional` to exist —
and it does not. `grep -rn "extend &" lang/std/` finds only `Equatable` and
`Comparable` forwarding in `lang/std/core/ref.ks`. *(Inferred from reading
plus that grep; not verified by running, since verifying it requires the
source change this write-up forbids.)* The `&` cases need either that stdlib
extension or an auto-deref rule at the condition — and, either way, a matching
deref in `lower_if`.

---

## 2. The eventual right shape: `conforms_to_resolved` on the type-infer side

Single source of truth means the analyzer must not own a second conformance
predicate. But it also must not force `TyVar` to become public. The shape that
satisfies both:

```rust
// kestrel-type-infer
pub fn conforms_to_resolved(
    ctx: &…, ty: &ResolvedTy, protocol: Entity, root: Entity, body_owner: Entity,
) -> bool
```

A `ResolvedTy`-shaped entry point that wraps `WorldResolver` internally,
keeping `TyVar` `pub(crate)`. `condition_check.rs` then deletes both private
helpers and calls it; `extern_ffi_safe.rs` can keep its own predicate but with
a comment saying *why* (see §4).

Not attempted here — it is a type-infer API addition, out of scope for a
blocked audit item, and it is pointless before the MIR prerequisite lands.

---

## 3. Sequencing: G13 comes first

**G13** (the `Copyable`/`Cloneable` extension-bound skip, being fixed
separately in `kestrel-type-infer/src/conformance.rs`) changes the predicate
G16 would route into. Sequence **G13 → re-evaluate G16**. There is no
file-level conflict — G13 touches `conformance.rs`, G16 would touch
`condition_check.rs` — so the two can be worked concurrently, but G16's
predicate choice should not be finalized until G13's answer is settled.

---

## 4. The widening: record `is_bool` and the FFI cousin as part of the item

Two things the audit entry did not capture, recorded here so a future fix
scopes correctly:

**`is_bool` is a second copy of the same mistake.** `condition_check.rs:143`
has the identical `let ResolvedTy::Named { … } = ty else { return false; }`
opener. It is what makes a bare `&Bool` (`j_ref_bool.ks`) a false E101 with no
user protocol anywhere in the picture. Any fix must cover `is_bool`, not just
`conforms_to_protocol`.

**`decl/extern_ffi_safe.rs:263-291` is structurally the same predicate and is
arguably correct.** `conforms_to_builtin_protocol(&HirTy)` rejects type
params, projections and function types with `_ => false`. Unlike G16 this is
defensible — FFI-safety needs a concrete layout, and an abstract position has
none. **Decision: leave it, but label it.** It should carry a comment saying
the `_ => false` is a deliberate exception to the "abstract positions MUST be
permitted" rule in `conformance.rs:125-129`, so the next reader does not
consolidate it into a shared helper and quietly make `extern` accept generic
parameters.

**Do not consolidate `parent_protocol_conformance.rs` into anything.**
`:245-247` documents that `ConformingProtocols`' transitivity would make that
analyzer a no-op. Verified verbatim in-tree. Any future "one conformance
predicate" refactor must preserve that carve-out.

---

## 5. NEW FINDING (not G16): `BooleanConditional` is analysis-only and silently miscompiles for concrete conformers

This is a **separate, un-filed bug**, discovered while diagnosing G16. It is
independent of G16's abstract-position problem and is strictly worse: G16
produces a spurious *error*, this produces *wrong output with a clean exit*.

**Severity: high, silent miscompile.** Suggested new audit entry.

`condition_check.rs:52` is the **only** reader of `Builtin::BooleanConditional`
in the entire tree (verified:
`grep -rn "BooleanConditional" lib/ --include="*.rs"` returns six hits — three
definitional in `kestrel-hir/src/builtin.rs`, two comments, and this query).
Nothing in hir-lower, mir-lower or codegen ever inserts a `boolValue()` call.
`lower_if` branches on the condition value directly.

**Repro, run at `6d0e30b1`** (scratchpad `g16/p_verify.ks`):

```kestrel
struct Inverted: BooleanConditional {
    var v: lang.i64
    func boolValue() -> lang.i1 { lang.i64_signed_gt(100, self.v) }
}
```

| value | `if x` took | `x.boolValue()` | |
|---|---|---|---|
| `Inverted(v: 5)` | TRUE | `true` | agrees by accident |
| `Inverted(v: 200)` | **TRUE** | **`false`** | **miscompile** |
| `Inverted(v: 0)` | **FALSE** | **`true`** | **miscompile** |

Wrong in both directions. MIR (`kestrel dump mir`, `Test.main`):

```
%v4 = copy_value %v3  // @owned Test.Inverted
branch %v4, bb1(%v2, %v4), bb2(%v2, %v4)
```

— a whole struct value handed to `branch`. The witness is present and
callable; three blocks later an *explicit* `x.boolValue()` lowers to
`%v33 = call Test.Inverted.boolValue(@borrow %v32)` followed by
`branch %v33`. Only the implicit path is broken.

**Why it has stayed invisible:**

1. **The only shipping conformer is `Bool`.** `grep -rn BooleanConditional lang/`
   finds the protocol (`lang/std/core/logical.ks:58-61`) and exactly one
   conformance (`lang/std/core/bool.ks:42`). `Bool` is a single-field
   `lang.i1` wrapper, so its raw representation *is* its `boolValue()` and
   branching on it raw is accidentally correct. Every other layout is wrong.
2. **Zero execution tests.** All 13 files under
   `lib/kestrel-test-suite/testdata/builtins/boolean_conditional/` are
   `// test: diagnostics` (verified). Two of them —
   `custom_type_in_if_condition.ks` and `optional_in_if_condition.ks` — are
   the exact miscompiling shape and are only ever type-checked, never run.

**Decision: fix the lowering first, then G16.** Concretely, in order:

1. Teach `lower_if` (and the `while` path, which desugars through
   `desugar_while` → `HirExpr::If`, and match-arm guards) to emit a
   `boolValue()` call when the condition's resolved type is not `lang.i1`.
   Decide there whether the insertion belongs in hir-lower (a `Sugar`-wrapped
   method call, visible to later analyses) or mir-lower — hir-lower is the
   better guess, since it keeps MIR's `branch` honest about only ever taking
   an `i1`.
2. Convert several of the 13 `boolean_conditional` testdata files to
   `// test: execution`, and add at least one whose `boolValue()` disagrees
   with its raw layout (the `Inverted` shape above) so a raw branch cannot
   pass.
3. Only then relax the E101 gate per §1/§2 — and add the `l_nobound.ks`
   negative test alongside, so the true positive is pinned before the
   permissive arms land.
