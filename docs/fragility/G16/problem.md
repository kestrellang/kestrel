# G16 — E101's condition test is a private `ResolvedTy::Named`-only conformance lookup

`medium` · `single-source-of-truth` · crate: `kestrel-analyze`
(`src/body/condition_check.rs`)

**Status: BLOCKED — deliberately not fixed.** See `decisions.md`. Fixing the
false positives in isolation converts six false errors into six silent
miscompiles, because `BooleanConditional` is never lowered (§3).

All observations below were re-verified at `6d0e30b1` with a
`cargo build --release --bin kestrel` binary unless explicitly marked
*(from prior diagnosis, not re-run)*. Repro files live in the session
scratchpad at `…/scratchpad/g16/`.

---

## 1. The two private predicates

`condition_check.rs` is a post-inference `BodyCheck`. Its module doc explains
why it is not a solver constraint: primitive `lang.i1` doesn't implement
protocols, so a `Conforms` constraint would fail for direct `i1` usage in
conditions. `kestrel-type-infer/src/generate.rs:465-472` confirms the other
half of that contract — the `HirExpr::If` arm generates **no** constraint on
the condition tyvar at all, deferring the whole question to this analyzer.

The analyzer resolves the protocol once (`:52-55`):

```rust
let bool_cond_protocol = cx.query.query(ResolveBuiltin {
    builtin: Builtin::BooleanConditional,
    root: cx.root,
});
```

**Verified: this is the only reader of `Builtin::BooleanConditional` in the
tree.** `grep -rn "BooleanConditional" lib/ --include="*.rs"` returns six hits
total: three in `kestrel-hir/src/builtin.rs` (the enum variant, its name
string, its `BuiltinKind`), one comment in `kestrel-hir-lower/src/desugar.rs:285`,
one comment in `kestrel-type-infer/src/generate.rs:472`, and this query. No
other consumer exists anywhere — not in hir-lower, not in mir-lower, not in
codegen.

Then two private helpers decide the answer. Both open with the same `else`:

```rust
// :142-153
fn is_bool(cx: &BodyContext<'_>, ty: &ResolvedTy) -> bool {
    let ResolvedTy::Named { entity, .. } = ty else { return false; };
    // …Intrinsic + NodeKind::Struct + Name == "i1"
}

// :156-169
fn conforms_to_protocol(cx: &BodyContext<'_>, ty: &ResolvedTy, protocol: Entity) -> bool {
    let ResolvedTy::Named { entity, .. } = ty else { return false; };
    let conforming = cx.query.query(ConformingProtocols { entity: *entity, root: cx.root });
    conforming.contains(&protocol)
}
```

Any `ResolvedTy` that is not `Named` falls through both and reaches the E101
push at `:124-135`. `ResolvedTy` has ten variants; only one is handled.

The diagnostic then renders through `describe_type` (`:172-209`), which *does*
have dedicated arms for `Param`, `SelfType`, `AssocProjection`, `Opaque`,
`Ref`, `Function` and `Tuple`. So the false error prints a clean, plausible
type name — which is exactly why it reads as intentional rather than as a
missing arm.

`kestrel-type-infer/src/conformance.rs:125-129` documents the opposite
contract for the same question, verbatim:

> `Param` / `SelfType` / `AssocProjection` / `Opaque` / `Function` / `AliasUse` /
> non-empty `Tuple` / `Infer` / `Error`: nothing concrete to disprove here, so
> permit. **Abstract positions MUST be permitted so generic bodies aren't
> spuriously rejected** (the conservative rule).

`condition_check.rs` rejects precisely the set that file says must be
permitted.

---

## 2. Six reproduced false positives

All six compile-error today; all six are legal Kestrel. Verified by running
`./target/release/kestrel build <file>` and reading the E101 label. (The
`E618: no entry point` that accompanies each is an artifact of these being
library-shaped probes with no `@main`, not part of the finding.)

| file | shape | observed E101 label |
|---|---|---|
| `a_param.ks` | `func f[T](flag: T) where T: BooleanConditional { if flag … }` | `expected Bool, found type parameter` |
| `b_self_ext.ks` | `protocol Truthy: BooleanConditional` + `extend Truthy { … if self … }` | `expected Bool, found Self` |
| `g_selftype_bound.ks` | `extend BooleanConditional { … if self … }` | `expected Bool, found Self` |
| `d_opaque.ks` | `-> some BooleanConditional`, then `if v` | `expected Bool, found some BooleanConditional` |
| `f_assoc.ks` | `type F: BooleanConditional` + `if h.flag()` | `expected Bool, found associated type` |
| `i_ref.ks` | `let r = &b.peek(); if r`, `peek() -> &Flag`, `Flag: BooleanConditional` | `expected Bool, found &Flag` |
| `j_ref_bool.ks` | same but `peek() -> &Bool` | `expected Bool, found &Bool` |

Two corrections to the shapes as originally filed:

- `g_selftype_bound.ks` renders `found Self`, not `found ?`. *(The prior
  diagnosis reported `found ?`; re-running gives `Self`.)*
- `f_assoc.ks` renders `found associated type`.

Control (`h_bool.ks`, `if b` with `b: Bool`) emits no E101 — verified.

### Two ways this is wider than the audit entry

**(a) `&T` is a sixth shape, and it goes through `is_bool`, not
`conforms_to_protocol`.** `j_ref_bool.ks` is a plain `&Bool` in an `if` — no
user protocol involved at all — and it is a false E101. The audit entry only
named `T` and `Self`.

**(b) `Opaque`, `AssocProjection` and `Ref` are affected too**, not just
`Param` and `SelfType`.

### The true positive that must survive any fix

`l_nobound.ks`:

```kestrel
func f[T](flag: T) -> lang.i64 {   // no bound on T
    if flag { 1 } else { 0 }
}
```

→ `error[E101] … expected Bool, found type parameter`. Correct today.
Verified. Blanket-permitting `ResolvedTy::Param` destroys this.

---

## 3. The blocker: `BooleanConditional` is never lowered

**This is a NEW finding the audit never filed, independent of G16's
abstract-position problem. It was re-verified by running, not taken on
faith.**

MIR's `lower_if` (`kestrel-mir-lower/src/body/control.rs:20-52`) lowers the
condition expression and emits `branch` on the resulting value directly:

```rust
let cond_val = self.lower_expr(condition);
…
self.emit_branch(cond_val, then_block, …, else_block, …);
```

There is no `boolValue()` insertion in `lower_if`, nor anywhere in hir-lower
or mir-lower — consistent with the grep in §1 showing `condition_check.rs` is
the only consumer of the builtin.

### Repro (written fresh, run at `6d0e30b1`)

`p_verify.ks` — a single-word conformer whose `boolValue()` is deliberately
the *inverse* of "payload is non-zero", so the two answers can be told apart
in both directions:

```kestrel
struct Inverted: BooleanConditional {
    var v: lang.i64
    func boolValue() -> lang.i1 { lang.i64_signed_gt(100, self.v) }
}
```

Observed output:

| value | `if x` took | `x.boolValue()` | |
|---|---|---|---|
| `Inverted(v: 5)` | TRUE | `true` | agrees, by accident |
| `Inverted(v: 200)` | **TRUE** | **`false`** | **miscompile** |
| `Inverted(v: 0)` | **FALSE** | **`true`** | **miscompile** |

Wrong in *both* directions. A two-word conformer (`n_verify.ks`,
`Flag { a: lang.i64, b: lang.i64 }`, `boolValue() = b > 0`) reproduces the
same way: `Flag(a: 99, b: 0)` takes the TRUE branch while `boolValue()`
returns `false`.

### MIR evidence

From `kestrel dump mir p_verify.ks`, `Test.main`:

```
%v0 = literal i64 5  // @owned Int64
%v1 = struct Test.Inverted { .0: %v0 }  // @owned Test.Inverted
%v2 = uninit Test.Inverted  // @owned Pointer[Test.Inverted]
store_init %v2, %v1
%v3 = begin_borrow_addr %v2, Test.Inverted  // @guaranteed Test.Inverted
%v4 = copy_value %v3  // @owned Test.Inverted
end_borrow %v3
branch %v4, bb1(%v2, %v4), bb2(%v2, %v4)
```

`branch` on a whole `Test.Inverted` struct value. Contrast the *explicit*
call in the same function, three blocks later:

```
%v33 = call Test.Inverted.boolValue(@borrow %v32)  // @owned Bool
branch %v33, bb4(…), bb5(…)
```

The witness exists and is callable; nothing calls it for the `if`.

### Why nobody noticed

**The only conformer that ships is `Bool`.** `grep -rn BooleanConditional lang/`
finds exactly two declarations: the protocol in `lang/std/core/logical.ks:58-61`
and `Bool`'s conformance in `lang/std/core/bool.ks:42`. `Bool` is
`struct Bool { private var value: lang.i1 }` — a single-field wrapper whose raw
representation *is* its `boolValue()`. Branching on it raw is accidentally
correct. Every user-defined conformer with any other layout is wrong.

Note the two-step: `if b` with `b: Bool` does **not** take the `is_bool` path
(`is_bool` requires the `Intrinsic` + `NodeKind::Struct` + name `"i1"` triple;
`std.core.Bool` is not intrinsic). It is admitted by `conforms_to_protocol`,
and then miscompiled-into-correctness by layout coincidence.

**And there is not one execution test.** All 13 files under
`lib/kestrel-test-suite/testdata/builtins/boolean_conditional/` are
`// test: diagnostics` — verified by `head -1` on each. Two of them are the
exact miscompiling shape and are never run:

- `custom_type_in_if_condition.ks` — `struct NonEmpty: BooleanConditional`
  with `var count: lang.i64`, `boolValue() = count > 0`, then `if items`.
  Compiled and type-checked only.
- `optional_in_if_condition.ks` — an `enum Option[T]: BooleanConditional`
  in an `if`.

### The interaction that makes G16 unfixable in isolation

The false E101 on abstract positions is currently **the only thing stopping
generic code from reaching that branch**. `func f[T](x: T) where T: BooleanConditional { if x … }`
is rejected at analysis time; if it were accepted, mono would emit a raw
`branch` on whatever `T` turned out to be. Relaxing the gate without teaching
`lower_if` to call `boolValue()` trades six false errors for six silent
miscompiles.

---

## 4. Family and blast radius

**Same-family private type-shaped conformance predicates:**

- `body/condition_check.rs:161` — `conforms_to_protocol` (G16 itself).
- `body/condition_check.rs:143` — `is_bool` (causes the `&Bool` case).
- `decl/extern_ffi_safe.rs:263-291` — `conforms_to_builtin_protocol(&HirTy)`,
  structurally identical: nominal arms query `ConformingProtocols`, everything
  else `_ => false`. Verified. Arguably **defensible** here — FFI-safety
  genuinely needs a concrete layout, and a type parameter has none — but it is
  currently indistinguishable from an oversight and should be labelled a
  deliberate exception.

**NOT the family** (entity-level `ConformingProtocols` use, which is the
query's correct shape) *(from prior diagnosis, spot-checked not exhaustively
re-run)*: `body/move_tracking.rs:1170`,
`decl/protocol_field_conformance.rs:52`, `decl/duplicate_callable.rs:199`,
`decl/type_alias_validation.rs:331`/`:519`,
`compilation/conformance_completeness.rs:682`/`771`/`1297`/`1909`/`1940`.

**Prior art any consolidation must preserve.**
`decl/parent_protocol_conformance.rs:245-247` carries an explicit warning —
verified verbatim:

> Do not use `ConformingProtocols` for the type here: that query is
> intentionally transitive, so it would treat inherited parents as already
> explicit and make this analyzer a no-op.

**Testdata blast radius is nil.** `grep -rl E101 lib/kestrel-test-suite/testdata/`
→ 0 files. E101 is exercised only by
`builtins/boolean_conditional/non_boolean_conditional_in_if.ks` via a loose
`// ERROR: condition` substring annotation.
