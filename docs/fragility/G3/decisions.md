# G3 — decisions

## 1. `FunctionKind`, not a new `ParamDef` field

**Decision: read the answer off `FunctionKind` via a new
`FunctionKind::takes_env_param()`.**

The obvious alternative was to add an explicit `is_env: bool` (or an
`is_synthesized` marker) to `ParamDef`, so the fact is carried in-band on the
parameter that has it. Rejected, for two reasons.

**It would be a second source of truth, and a settable one.** Every producer of
a `ParamDef` would have to remember to set it, and `ParamDef::new` — used in a
dozen places across mir-lower and the MIR passes — would either need a new
argument at every call site or a default that is silently wrong for the two
producers that matter. A field you can forget to set is exactly the shape of the
bug being fixed.

**The kind already answers it, exactly.** Verified by reading both producers:

- `kestrel-mir-lower/src/body/closure.rs:226-234` picks
  `FunctionKind::ClosureCall { env_struct }` when the closure captures, and
  `FunctionKind::Closure { parent_func }` when it does not. `:274-283` pushes the
  `"env"` `ParamDef` at index 0 **outside** that match, so it runs for both arms
  unconditionally.
- `passes/thunk.rs:89` sets `FunctionKind::Thunk { original }` immediately before
  its own unconditional `"_env"` push at `:102-107`.

There is no third producer of a leading env pointer and no arm of these two that
skips the push. `matches!(kind, Closure | ClosureCall | Thunk)` is therefore
*equivalent* to "params[0] is the env pointer", not merely correlated with it.

**Precedent was already in-crate.** `mono/collect.rs`'s
`detect_implicit_protocol` was asking the same question with that same inline
`matches!`, for the same reason ("their first param is env pointer, not Self").
Two call sites re-deriving one predicate is how the predicate drifts, so
`detect_implicit_protocol` now calls `takes_env_param()` too. The method is the
single place the question is answered.

**Guarded from the other side.** A `debug_assert!` in the thunk pass checks that
when the kind promises an env parameter, `params[0].name` is still `"env"` or
`"_env"`. The names are no longer *load-bearing*, but they remain a cheap
tripwire: if someone later reorders `closure.rs` so the env param is not at
index 0, debug builds fail loudly instead of quietly reintroducing this bug in
mirror image.

## 2. No `self` special case — the structural fix subsumes it

**Decision: delete the `p.name != "self"` filter outright and add nothing in its
place.** Forwarding is `.skip(if needs_env { 1 } else { 0 })` and nothing more.

The filter existed to strip a method's receiver. The reasoning was that a method
used as a function value would otherwise forward its `self` parameter. Two
questions had to be answered before removing it, and both were checked by
reading *and* by compiling probes:

**Does `instance.method` (a bound method value) reach `ApplyPartial`?** No.
`hir-lower/expr.rs:54-67` lowers a standalone `AstExpr::MemberAccess` to
`HirExpr::Field`, and `:271-284` does the same for a multi-segment path whose
head is a local — so `b.doubled` is a *field access*, never a function
reference. Confirmed empirically: `apply(b.doubled, 7)` is rejected with
`error[E100]: method 'doubled' on 'Box' must be called` / "primitive methods
cannot be used as first-class values". No MIR is produced.

**Does a free function with a parameter spelled `self` reach it?** Yes — that
was the only live trigger for the filter, and it was a bug, not a feature.
`func combine(self: Int64, x: Int64)` is legal (see below) and was being
silently stripped of its first parameter. Under the structural fix it is
forwarded like any other parameter and variant (f) now returns `307`.

### 2a. `self` is not a keyword — the addendum's claim was wrong

`docs/fragility-audit-addendum.md:203` asserted the `p.name != "self"` twin was
"safe only because `self` is reserved". It is not reserved. The lexer and parser
accept `self` as a parameter name, and `func combine(self: Int64, x: Int64)`
compiles end to end. That sentence has been corrected in place.

### 2b. FOUND FALSE: `Type.instanceMethod` **does** reach `ApplyPartial`

The design also asserted that `Type.method` is excluded by
`kestrel-name-res/src/resolve_value.rs:400-442`'s `is_static_method` filter
(`ctx.has::<Static>(e) || callable.receiver.is_none()`), and therefore that
`FunctionKind::Method` can never be an `ApplyPartial` target.

**That is false.** Probed directly:

```kestrel
struct Box { var v: Int64;
  func doubled(x: Int64) -> Int64 { self.v * 100 + x }
}
func apply(f: (Int64) -> Int64, a: Int64) -> Int64 { f(a) }
// ...
let r = apply(Box.doubled, 7);
```

This compiles, and the MIR shows a `FunctionKind::Method` reaching the pass:

```mir
; function: Test.main
    %v0 = apply_partial Test.Box.doubled.thunk()  // @owned @thick (Int64) -> Int64

; function: Test.Box.doubled.thunk
bb0(%v0: @owned Pointer[()], %v1: @owned Test.Box, %v2: @owned Int64):
    %v3 = call Test.Box.doubled(%v1, %v2)
```

The thunk is a 2-real-parameter function (`Box`, `Int64`) standing behind a
thick type that declares **one** parameter — an unbound method reference that
the front end never rejected and never bound a receiver to. `Box.doubled` as a
value is a broken feature independent of G3.

**This changes its failure mode, and that is recorded, not fixed.** Before G3,
the `p.name != "self"` filter stripped the receiver, the thunk called a 2-param
target with 1 argument, and the build failed loudly (backend verifier error;
with G3's new `verify_ossa` arity check in place, an ICE naming
`Test.Box.doubled.thunk`). After G3 the thunk is internally consistent — 2 args
to a 2-param target, so both the new count check and the new type check pass —
and the mismatch moves to the *indirect* call through the thick value, where no
check can see it. It became a silent miscompile: `apply(Box.doubled, 7)` printed
`r=4354120236`.

The filter was masking this by accident, for a wrong reason, at the cost of
miscompiling variants (a)/(b)/(c)/(f) of ordinary user code. Restoring it is not
an option. The correct fix is in the front end — reject `Type.instanceMethod`
used as a value with the same E100 that already rejects `instance.method` — and
that is out of G3's scope and outside its file allowlist. **Escalated, not
acted on.** See §4.

## 3. Two backstops, at different levels, for different reasons

**In the thunk pass: count *and* per-position type.** The type half is not
belt-and-braces — it is the only one of the two that catches G3's actual repro,
where `needs_env` firing and the filter dropping the same parameter made the
count coincidentally correct. It is exact rather than heuristic because the
compared `TyId`s are the same interned ids cloned straight off
`target_func.params`, before monomorphization, so there is no substitution,
ref-decay or ByVal/ByRef gap to reason about. `debug_assert!` is appropriate
here: this is an invariant of code the pass just generated itself, so a release
build cannot encounter it without a debug build encountering it first.

**In `verify_ossa`: count only, and only for statically resolvable callees.**
`VerifyModule::callee_declared_arity` returns `Some(n)` for `Callee::Direct`
(`MirModule`) and `Callee::Resolved` (`MonoModule`), `None` for `Thin`, `Thick`
and `Witness`. Deliberately **not** extended to per-argument types: at that
level the arguments have been through substitution, ref decay and
by-value/by-reference selection, and a naive `TyId` comparison would fire
corpus-wide on correct code. Arity is the one property that survives all of
those transformations intact.

This is a real verifier error rather than a `debug_assert!` because unlike the
thunk-local check it guards code that *other* passes produced, and because its
whole purpose is to convert an opaque late-stage failure (Cranelift's
"mismatched argument count for `v3 = call fn0(v2)`", with no Kestrel function
name and no span) into a located compiler error naming the offending function.

### 3a. The new check found a real inconsistency immediately

Two `kestrel-mir` unit tests failed on the first run:
`passes::drop_shim::tests::deinit_only_shim` and
`::struct_deinit_then_field_drops`, both with
`call passes 1 argument(s) to a callee declaring 0`.

Diagnosed, not silenced. Both fixtures stub a deinit with
`FunctionDef::new(deinit_entity, "…deinit", unit_ty)` — **zero parameters** —
while a real deinit declares `mutating self` (`kestrel-mir-lower/src/items/function_sig.rs:89-103`
pushes a `"self"` `ParamDef` with `ParamConvention::MutBorrow` for any callable
with a receiver, and forces `MutBorrow` for `NodeKind::Deinit` specifically).
The drop shim correctly forwards a mut borrow to it; the *fixtures* were
building an internally inconsistent module and then asserting it verifies clean.

The fixtures were corrected to declare the `self` parameter they would really
have. The check was not weakened. This is the only place outside G3's file
allowlist that was touched.

## 4. Open — recorded, not acted on

### 4a. `instance.zeroArgMethod` silently *calls* instead of referencing

**Still open. Reproduction re-confirmed 2026-08-20** against
`target/release/kestrel` with the §4c fix in place — this is a distinct path
and is unaffected by it.

```kestrel
struct Box {
    var v: Int64;
    func zero() -> Int64 { self.v }
}
let b = Box(v: 3);
let g = b.zero;      // no `()`
print("g=\(g)");     // prints `g=3` — the Int64, not a function value
```

`instance.method` lowers to `HirExpr::Field`, inference emits a `Member`
constraint with zero arguments, and for a method with a required parameter that
hard-errors (E100, §2). For a **zero-parameter** method there is nothing to
mismatch, so the constraint unifies with the method's *return* type and the
member is invoked. The user asked for a function value and got a call.

Why §4c's fix does not cover it: that check lives in `lower_path`, and a
`local.member` path never reaches the `ResolveValuePath` fallback — `lower_path`
peels the leading local at `expr.rs:274-284` and emits `Local` + `Field`, so
there is no entity in hand to test. The fix has to be on the inference side, in
`solve_member`: a `Member` constraint arising from a *bare* field access (no
`MethodCall`) that lands on a `Callable` should be `MethodNotCalled` regardless
of parameter count, instead of succeeding when the argument list is empty. That
is one line of policy in the solver, but it needs a sweep of the corpus first —
`b.zero` returning a value may already be relied on somewhere.

### 4b. Real bound-method values would need a third case in `thunk.rs`

Making `instance.method` produce a genuine function value means synthesizing an
env struct that holds `self` and emitting an `ApplyPartial` that captures it —
a third shape alongside "capture-free thunk" and "closure with captures".
That is a feature, not a fix.

### 4c. `Type.instanceMethod` as a value (§2b) — **FIXED** (follow-up, uncommitted)

Closed in HIR lowering, not name resolution. Details in §5.

## 5. Follow-up: closing §4c in HIR lowering

### 5.1 Why `is_static_method` never saw it

The G3 design blamed the wrong filter. `resolve_value.rs`'s `is_static_method`
(`:400-403`) is real and does work — it is why `Box.extensionInstanceMethod`
already fails with *"undefined name 'Box.tripled'"*. But it is only consulted
from `resolve_extension_static_method`, which `walk_path_from` reaches **only
as a fallback**, after the direct-children lookup has come up empty:

```rust
// resolve_value.rs:300-327 — walk_path_from
let children = ctx.query(VisibleChildrenByName { parent: current, name: segment, .. });
if !children.is_empty() {
    if is_last { return classify_value_results(ctx, children); }   // ← no static filter
    …
}
// No direct children — try extension static methods (only for last segment)
if is_last && ctx.has::<Typed>(current) && let Some(result) =
        resolve_extension_static_method(…) { return result; }      // ← filter lives here
```

`doubled` **is** a direct child of `Box`, so the walk returns it at the first
branch and the filter is never reached. The leak is exactly the set of
instance methods declared in the type's own body; extension-declared ones were
never affected.

### 5.2 Where the rejection went, and why not name-res

**Rejected: filtering the direct-children branch.** That branch also serves
enum cases, nested types, static vars and module members, none of which are
`Static` functions, so the filter would have to be a keep-list rather than the
drop-list `is_static_method` provides — the F10 mistake (see the comment on
`resolve_extension_static_method`) in a new place. And it degrades the message
to "not found (failed at 'doubled')", which is what `Box.tripled` prints today
and is actively misleading.

**Rejected: post-solve validation in type-infer.** `validate_ref_placement`
(`solver.rs:490`) is the standing precedent for "this expression is fine except
as a value", and it already carries the `direct_callee_exprs` exemption set. But
`Type.instanceMethod` is not a *typing* mistake — the form is wrong before any
type is known — and reporting it there means the E100 for the call form and the
E100 for the value form live in different crates, drifting apart.

**Chosen: `lower_path` in kestrel-hir-lower.** The call form
`Box.doubled(b, 7)` was *already* rejected there — `lower_call:691` asks
`is_instance_method_on_type` and emits "instance method 'doubled' cannot be
called on a type". The value form is the same rule with no call, so it now
shares the same predicate and the same emitter:

- `LowerCtx::is_instance_method(entity)` — the single definition of "Function,
  has a receiver, not `Static`". `is_instance_method_on_type` was rewritten to
  call it instead of re-deriving it inline.
- `LowerCtx::emit_instance_method_on_type(method, span, MethodOnTypeUse)` — one
  diagnostic, `E100`, two wordings (`Call` / `Value`). The call form's message
  is unchanged; it merely gained the `E100` code it should always have carried
  (`InferError::InstanceMethodAsStatic`, the solver's twin of the same message,
  already renders E100).

The value-form check keys off the resolved **entity**, not the segment shape,
so it covers structs, enums and protocols uniformly. `P.hop` used to reach
post-mono as an unresolved `ApplyPartialTarget` and **ICE**; it now reports.

### 5.3 The callee exemption is load-bearing

`lower_call` falls through to `self.lower_expr(body, callee)` for callees it
did not special-case, and that lands in `lower_path`. Without an exemption the
new check would fire on legitimate call syntax that reaches the fallthrough
(and on `Self.method()`, whose `SelfValue` prefix `is_instance_method_on_type`
deliberately does not match). So `lower_call` routes callees through
`lower_callee`, which sets `LowerCtx::in_callee_position`; `lower_path`
**consumes** the flag with `mem::take` at entry, so it protects exactly one
path — the callee itself — and never the paths nested inside it.

### 5.4 Verified

`apply(Box.doubled, 7)` printed `r=4311325036` before and now reports
`error[E100]: instance method 'doubled' cannot be used as a value`. Static
methods as values (`apply(Box.stat, 7)` → `307`, including the
`extend`-declared `Box.extStat`) still work, as does `Box.stat(7)`. The four G3
repros (`a_env_first`, `b_underscore_env_first`, `c_env_second`,
`f_self_param`) all still print `307` and exit 0.

Testdata added under
`lib/kestrel-test-suite/testdata/expressions/calls/method_calls/`:
`instance_method_on_type_as_value.ks` (struct + enum + protocol, diagnostics),
`static_method_on_type_as_value.ks` (execution, the must-keep-working half),
and `instance_method_reference_without_call.ks` (pins the `instance.method`
E100 on a *struct* receiver — `primitive_methods_errors.ks` only covered the
primitive-receiver path).

---

## 5. Correction (2026-08-20): the per-position type check was too strict at index 0

§3's claim that "the compared `TyId`s are the same interned ids cloned straight
off `target_func.params`" is true for every REAL parameter and **false for the
env pointer at index 0**. As shipped, the check panicked on *every* stdlib
program under a debug-built compiler:

```
thread 'main' panicked at lib/kestrel-mir/src/passes/thunk.rs:210:
std.collections.Array.subscript.closure.3.thunk: forwarded arg types do not
match the target's params (forwarded [TyId(488)], expected [TyId(509)])
```

Resolving both ids through the arena settles it — this is not `TyId` interning
noise, the two types are genuinely different and are *supposed* to be:

| side | `MirTy` |
| --- | --- |
| thunk's forwarded arg 0 | `Pointer(Tuple([]))` — i.e. `Pointer[()]` |
| target's `params[0]` (`env`) | `Pointer(Named { entity: 2147483644, type_args: [TyId(3)] })` |

The thunk's env param is **deliberately type-erased**. A thunk is what an
indirect closure call lands on, so its signature has to be uniform across every
closure; `run_thunk_pass` has built `env_ty = ty_arena.pointer(unit_ty)` since
the pass was introduced (`336f577b`). The target closure declares the concrete
environment it wants — `Pointer[<synthesized env struct>]` (mir-lower
`closure.rs`, `env_ty`), or, for a boxed closure, the box binding's `raw_ty`,
which `closure_box.rs::unwrap_handle_to_pointer` guarantees is also a
`MirTy::Pointer`. Two different pointees, one machine word; codegen
reinterprets. So index 0 can never satisfy `TyId` equality.

**Fix.** Split the check rather than delete it:

- index 0 under `needs_env` — assert the target's env param is *pointer-shaped*
  (`matches!(.., MirTy::Pointer(_))`). That is the strongest property that is
  actually invariant there.
- every position after it — keep the exact `TyId` equality, which really is
  exact.

**This still catches G3.** Verified by re-introducing the old name sniff
(`needs_env = params[0].name == "env" | "_env"`) and compiling
`codegen/closures/env_named_param_used_as_value.ks`:

```
Test.combine.thunk: target's kind Some(Free) promises a leading env pointer but
params[0] `env` is Named { entity: Entity(2942), type_args: [] } — the thunk
forwards a type-erased `Pointer[()]` into that slot, so a non-pointer there is
a miscompile
```

G3's repro passed an `Int` in the env slot, and `Int` is not a pointer. What the
weakened check can no longer distinguish is a wrong-slot forward into a param
that is *itself* a pointer; the arity check plus the kind-derived `needs_env`
(the actual G3 fix) covers that shape.

### 5a. `debug_assert!` was the wrong severity — it is now a hard `assert!`

§3 argued "a release build cannot encounter it without a debug build
encountering it first." That is exactly backwards for this repo. **Nothing in
CI builds a debug compiler and compiles Kestrel with it**, so a `debug_assert!`
in the pipeline is closer to dead code than to a backstop:

- `.github/workflows/ci.yml` runs `cargo build --workspace` (debug) but never
  *invokes* the resulting `kestrel` on any `.ks` file.
- the same job runs `cargo test --workspace --exclude kestrel-test-suite` — the
  `.ks` suite is explicitly excluded (`47d2714b`), and its own comment already
  concedes "some cases hit debug-only rowan asserts that don't fire in release."
- the `bootstrap` job builds with `profile: release`.
- `triage` builds `--release` per `.triage/config.toml`.

So the release corpus is exercised ~3800 ways and the debug compiler is
exercised zero ways. Same shape as F8 ("the suite never runs LLVM"). Both thunk
checks are therefore plain `assert!` now — they are O(params) once per thunk,
and being live in release is what makes them a backstop at all. The full suite
(3795 passed, 0 failed) is now real evidence that they hold corpus-wide, which
it was not before.

This is the third recorded debug-only compiler panic
(`Slice.first` / `a30332b2`, `Pointer[CopyableStruct]` / `ty_query.rs:174`), and
the second where the assert itself encoded a false invariant. The pattern is
consistent enough to be worth a rule: **do not add `#[cfg(debug_assertions)]`
invariants to the compiler pipeline — either the invariant is worth checking in
release, or it is not worth checking.**
