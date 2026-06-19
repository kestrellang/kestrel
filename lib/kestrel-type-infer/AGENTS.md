# kestrel-type-infer — patterns

## Adding a new `InferError` variant

A new variant must be mirrored in **five** files — miss any and the build fails with non-exhaustive-match errors only after compiling a dependent crate, which is slow to discover.

1. **`lib/kestrel-type-infer/src/error.rs`**
   - Add the variant to `pub enum InferError`.
   - Add the span arm in `impl InferError::span()`.

2. **`lib/kestrel-type-infer/src/result.rs`** — `describe_error()` match arm (short one-liner used as the `detail` string).

3. **`lib/kestrel-compiler/src/diagnostic.rs`** — match arm on `InferError` that builds the user-facing `Diagnostic` (message + labels + notes).

4. **`lib/kestrel-analyze/src/body/type_check.rs`** — `format_error()` match arm returning `(message, label_text)`.

5. **`lib/kestrel-compiler-driver/src/lib.rs`** — both `describe()` (short name) and `format_error()` (debug-log string).

## Reporting diagnostics from the solver

- Use `ctx.report_error(InferError::...)` — never `qctx.accumulate(Diagnostic::...)`. The accumulate path is reserved for hir-lower / decl-level analyzers. Solver errors flow through `InferError` so cascades get absorbed via `TyKind::Error`.
- `report_error` returns an Error TyVar; use it as the result of the constraint-generation branch so downstream constraints see the absorber.

## Memberwise init validation

`gen_struct_init` has a memberwise path that zips args against fields — validate arity and labels **before** the zip, because zip silently truncates. Filter fields by `NodeKind::Field` AND *absence* of the `Computed` marker; computed properties share `NodeKind::Field` but aren't memberwise-init storage.

## Argument → parameter binding

Mapping a call's arguments onto a callable's parameters has **one** source of truth: `kestrel_ast_builder::arg_binding::bind_arguments` (with the `binds` yes/no helper). Arguments bind in declaration order; defaulted parameters may be skipped **anywhere** (leading, middle, trailing), not just at the end.

**Never reimplement positional `arg[i] ↔ param[i]` label matching.** It was a recurring bug: a positional zip can only line provided args up with the *first N* parameters, so a call skipping a non-trailing default (`zdt.adding(months: 1, days: 10)`) failed with a spurious "no member" / "wrong argument label" / "no matching overload". The binder unified **five** sites; any new call-resolution path must route through it:

- overload selection — `resolve.rs::matches_labels`, `constraint.rs::labels_match`
- label/arity validation + arg-type coercion — `solver.rs::solve_member`, `emit_resolved_call`, `types_compatible` (build the plan via `binding_plan_for`)
- default-fill + value ordering at lowering — `kestrel-mir-lower`'s `lower_call_args_bound` (NOT the old trailing-only `expand_default_args`)

When 2+ candidates pass label filtering, type disambiguation also runs through the plan (`types_compatible`), and default slots are skipped. Coerce/validate only the `Binding::Arg` slots; defaulted-and-skipped slots are checked at the default's definition site.

It is a **pure function, deliberately not a query**: its input includes the call site's argument labels (ephemeral, high-cardinality), so memoization would never hit; the cacheable part (resolving the `Callable`) is already query-backed via the entity.

(Memberwise struct init — see above — is a *separate* positional zip over fields and does not go through the binder; fields have no skippable defaults.)

## Structural / intrinsic types extend via synthetic `lang` entities

`()`, `!`, and `lang.*` intrinsics are extendable (`extend (): P`,
`extend lang.i64 { }`). They have no nominal entity of their own (or it's an
`Intrinsic` Struct), so the Entity-keyed conformance/member pipeline routes them
through synthetic `lang` entities: `lang.()` / `lang.!` (seeded in
`kestrel-ast-builder/src/lang_module.rs`) and the existing `lang.iN`.

**Invariant:** any conformance/member lookup keyed on a type's entity must first
map a structural/intrinsic type to its synthetic entity, or it silently fails
(`E100 () !: P`, `no member`, or downstream `Callee::Witness not resolved`). In
this crate the two sites are `resolve.rs::conforms_to` and
`resolve.rs::resolve_member`: when `TyKind::entity()` is `None`, match
`TyKind::Tuple([])` → `"()"` / `TyKind::Never` → `"!"` and resolve via
`kestrel_name_res::extensions::resolve_lang_child(ctx, root, name)`. The same
mapping lives in name-res `ExtensionTargetEntity` (`AstType::Unit`/`Never`) and
mir-lower `try_lang_primitive` (entity → `Tuple([])` / `Never` / `I64`) — **all
sites must agree**, so when you add a new Entity-keyed conformance/member path,
add the mapping too.

**References (`extend &T: P`) join this pattern with GENERIC entities**:
`lang.&` / `lang.&mutating` carry one type param (`T`, the pointee — extension
LHS args bind BY NAME to the target's declared params, so ref extensions must
spell the pointee `T`). The reverse detector is
`kestrel_name_res::extensions::lang_ref_mutability`. The mapping-site list for
refs: `conforms_to`'s `TyKind::Ref` arm (declared check — the pointee is an
opaque TyVar there), `conformance.rs::type_satisfies`' Ref arm (routes to
`nominal_satisfies(amp, [pointee])` so extension `where T: P` bounds evaluate
at the real pointee), `lib.rs::create_extension_self_type` (Self inside a ref
extension is `TyKind::Ref{Param}` — a leaked entity type makes extension
bodies dispatch onto themselves and recurse), mir-lower
`try_lang_primitive`/`build_self_type` (→ `MirTy::Ref`), and mono
`match_pattern`'s Ref arm (exact mutability — NO `&mutating` ← `&`
subsumption anywhere). The type layer must NEVER see the entities as `Named`.

## Ref decay (`&T → T`) has FIVE value-position sites

A ref-returning call's result is pinned to its pointee (copy/clone) only in
value positions. There is **no** decay logic on the constraint itself —
`bind_call_result` (solver.rs) forces `result ≡ pointee` iff the call's
`HirExprId` was recorded in one of the decay-position sets during constraint
generation (generate.rs). Outside those sets the result stays `&T`. The five
sites:

| Position           | Set                  | Recorded at (generate.rs) |
|--------------------|----------------------|---------------------------|
| match scrutinee    | `scrutinee_exprs`    | the `Match` arm           |
| let-binding init   | `binding_init_exprs` | the `Let` stmt arm        |
| assignment target  | `assign_target_exprs`| the `Assign` arm          |
| if/match arm value | `always_decay_exprs` | `mark_arm_value`          |
| **return tail**    | `always_decay_exprs` | the tail-expr block       |

**Invariant:** every position where a ref-returning call's value is *consumed
as its pointee* must record its expr id, or the recorded `expr_types[expr]`
stays `&T` and a downstream consumer surfaces `expected T, got &T`
(order-dependent — `bind_call_result` may run before the coerce). `let v =
call(); v` working while `return call()` failed (bug B1) was exactly a missing
site. Reuse `mark_arm_value` — it recurses through `Block` wrappers and is inert
for non-call tails.

**The gate is the declared/target type, not the position.** `bind_call_result`
decays *unconditionally* once an expr is in a set, so a position whose target may
legitimately be `&T` (return tail of a `-> &T` fn) must NOT record the expr when
the target resolves to `TyKind::Ref` — else the ref is wrongly peeled and
mismatches. The return-tail site gates on `ctx.return_ty` being non-ref;
`if`/`match` arms cannot carry refs across a merge by design, so they always
decay.

## Synthetic-span diagnostics fail SILENTLY

A solver error whose span is `Span::synthetic(0)` renders as NOTHING in
the CLI — the build fails with no output and no binary, which reads as
success to anything grepping stderr. Every emitted constraint that can
ERROR must carry a real span: for solver-side type formations use the
formed `HirTy`'s own span (the declared-signature site renders fine even
when it points into stdlib — `emit_static_wellformedness` in solver.rs
is the precedent). When probing compiler behavior from the CLI, verify
the OUTPUT EXECUTABLE exists; never conclude "compiles" from empty
stderr.

## Copy semantics: never re-implement the fold

The copy-semantics decision tree lives in `kestrel-copy-fold`
(`instance_semantics` / `fold_members`); this crate's `SolverCopyLayer`
(solver.rs) is one adapter. Never re-implement the gating fold or the tuple
fold in solver code. Any deliberate divergence from the kernel rule must carry
a `TODO(copy-drift #n)` comment at its classifier arm — never converge or
introduce one silently.
