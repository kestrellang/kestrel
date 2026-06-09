# Stage 1 — Semantics

## Decided

### The root rule (escape proof)

Every MIR value carries `root_provenance`, stamped at creation and copied
O(1) through projections: `Param(idx) | Static | Local | PointerDerived`.
At a `Return` of a ref-typed value: the root must be `Param` (a mutable one
for `&mutating`), `Static`, or `PointerDerived`; `Local` is the escape
error. No region/outlives inference exists or is needed (`references.md`
§5 #3 — deferred entirely).

### PointerDerived = "inherits the pointer's contract"

`pointer.value` lowers as an ordinary borrow of the receiver pointer; the
result's root is `PointerDerived`, and it may escape: the reference is
exactly as trustworthy as the pointer it came from — the same trust point as
`Pointer.read()`, returning a view instead of a copy. References that never
touch `Pointer` remain fully verified. Why this cannot be a *checked* root
short of Stage 2: `references-gaps.md` §10.3.

### Caller-side scoping

A ref-returning call's result is registered `@guaranteed` with
`borrow_source` = the unique borrowable argument (usually the receiver).
Its life is bounded by the full expression; `set_terminator` still ends all
borrows at block exits (the callee carves out exactly the one returning
value).

### Mutability

- `&mutating T → &T` coerces one-way (`solve_coerce` arm; Ref/MutRef still
  never unify). At MIR it is a bit-copy + provenance carry — free because
  may-alias means there is no loan to suspend (`references-gaps.md` §10.1).
- May-alias holds everywhere; Check 5 is relaxed per the standing decision.
- **Accepted behavior, recorded:** `f(arr.at(0), arr)` can make a write
  through the mut-ref observable via the copy (a COW value-semantics
  violation) — the same class the no-exclusivity decision accepted for
  Design B (§10.4).

### Return ABI

`ret_borrow` functions return the raw pointer (`ReturnMode::Direct(ptr)`).
The scalar load-through in `resolve_scalar` must be bypassed in **both**
backends, or the pointee is silently returned by value.

---

## DECIDED — Q8: transparent place (option (a), 2026-06-09)

**One rule:** a ref-typed expression is a **place**, never a first-class
value. *Place contexts* use it in place; *value contexts* read it
(copy-out). Nothing of type `&T` is ever stored, and refs have no identity —
there is no way to compare, capture, or name a reference itself in stage 1.

### Place contexts (the ref is used in place — no copy)

- **Receiver see-through:** a `&T` / `&mutating T` expression may be the
  receiver of every member-shaped operation — field access, method call,
  paren-subscript, operators (`ProtocolCall` desugar), `for-in`, compound
  assign, string interpolation. Resolution peels the ref and does nominal
  lookup on `T`; the ref passes directly as the `@guaranteed` receiver.
- **Receiver convention matrix:**
  - borrowed-`self` method: OK on `&T` and `&mutating T` — no copy.
  - `mutating` method / setter base / compound-assign target: requires
    `&mutating T`; on shared `&T` it is the const-cast error (the new
    shared-ref class in `classify_mutability` — see `errors.md`).
  - `consuming` method: **not** a place context — falls through to copy-out
    below (read the place, consume the copy). No new error needed.
- **Borrow-convention argument position** *(decided 2026-06-09 — borrow
  args are place contexts, not value contexts)*: a `&T` / `&mutating T`
  expression passed where the param convention is borrow (`x: T`) passes
  the referent place directly as the `@guaranteed` argument — no copy, no
  clone; NotCopyable pointees are legal. (`&mutating T` first coerces via
  §10.1 — bit-copy, free.) A `mutating x: T` argument position likewise
  passes the place and requires `&mutating T`; a shared `&T` there is
  E-REF-20 (already in its trigger list). Receiver position and argument
  position are thus symmetric: `box.peek().count()` and
  `count(box.peek())` both borrow in place.
- **Write-through:** mutating a `&mutating T` place writes the referent —
  `arr.at(i).increment()` mutates the element. Whole-place assignment
  (`arr.at(i) = v`) additionally needs call-as-place grammar → stage 1.5.

### Value contexts (copy-out)

Using a ref where an owned `T` is expected — argument to a `consuming`
param, `consuming` receiver, assignment RHS, `return` of `T`, `match`
scrutinee —
**reads the place**: Copyable copies, Cloneable clones (identical to how
borrowed-param reads behave today via CopyValue→clone in mono expand),
NotCopyable is rejected by the existing copy guards (wording extended to
name the reference). Consequences:

- **Binding decay** *(derived rule — flag for maintainer veto)*:
  `let x = ring.peek();` is **not** an error; the binding stores, storing is
  a read, so `x` gets the decayed owned `T`. In a COW language the copy is a
  retain — cheap by design. Stage 1.5's `let r = &expr;` is the explicit
  opt-in that *keeps* the ref; `let r: &T = …` annotations stay rejected by
  the stage-0.5 type-position walk. (Alternative if vetoed: restore the
  E-REF-14 hard error.)
- **Match dissolves:** `match r` copies the scrutinee out; a ref never
  enters the match machinery, so the planned ref-scrutinee ban (E-REF-18)
  is unnecessary.
- **Merges:** in value context a ref decays *before* a block merge, so only
  owned values cross merges; a ref used as a *place* across a merge remains
  the E-REF-15 error.

### Stage-1 shallowness (deliberate)

`r.field` in value context types as owned `F` (copy-out) — same as a
borrowed-param field read today. Ref-typed *projections* (`r.field : &F`,
true deep place chains) defer to stage 1.5 alongside named bindings, where
keeping place-ness past one expression starts to matter.

### Implementation anchors — verified 2026-06-09

The cost basis for the decision: ~2-4 wk, concentrated at 4 sites:

1. **One peel.** All member dispatch — field, method, paren-subscript,
   operators (desugared to `ProtocolCall`), for-in, compound assign, string
   interpolation — funnels through `solve_member`
   (kestrel-type-infer/src/solver.rs:2535), and the receiver `TyKind` is
   extracted at a single point (~:2554) before nominal lookup. Peeling
   `Ref`/`MutRef` there covers every dispatch form at once.
2. **MIR lowering is already transparent for borrows.** A `&T` expr at MIR
   is a `@guaranteed` value — exactly how borrowed params work today. Field
   access on non-var bases already routes `lower_expr_for_borrow` →
   `emit_struct_extract`; receivers via `prepare_call_arg_for_expr`. A
   ref-typed expr is "a borrowed param that travels"; no new lowering.
3. **Two coercion arms** in `solve_coerce` (solver.rs:1295):
   `&mutating T → &T` (§10.1, needed by (c) too) and `&T → T` copy-out.
   Copy-out rides the existing CopyValue→clone mono-expand machinery:
   Copyable copies, Cloneable clones (matches borrowed-param reads today),
   NotCopyable rejected by the existing copy guards. The copy-out arm must
   be **convention-aware** (borrow-args decision above): it must *not*
   fire when the expected position is a borrow- or mutating-convention
   parameter — those route the existing borrow argument path
   (`prepare_call_arg_for_expr`), passing the place. This is the one new
   solver thread beyond the original costing (small bump): the param
   convention must be consulted at the call-constraint site; #106 put
   conventions in the type system, so the information is available there.
4. **One mutability classifier.** `classify_mutability`
   (kestrel-analyze/src/body/access_mode.rs:249) feeds E203-E206 and already
   has the precedent carve-out (#106: a `MutBorrow`-conventioned closure
   param is a mutable place without `var`). Refs generalize it from "param
   convention" to "expression type": `&mutating T` → Mutable, `&T` → new
   shared-ref class with its own message. Today a ref-typed call result
   would fall to `_ => Temporary` (wrong-but-safe E205 wording). Note this
   classifier is currently purely syntactic (HirExpr shape); consulting the
   inferred type is the one new thread.

Caveat: the `codegen_byref_scalar_deref` known bug (@guaranteed scalar
double-deref) becomes load-bearing — **fix it before the see-through
lands**. The bulk of the work is the test surface
(operators/subscripts/interpolation/for-in through refs), not the compiler
change.
