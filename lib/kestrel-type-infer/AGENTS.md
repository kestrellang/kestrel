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
