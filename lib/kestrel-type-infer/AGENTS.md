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
