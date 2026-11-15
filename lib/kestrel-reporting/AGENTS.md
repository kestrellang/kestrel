# kestrel-reporting

Prints diagnostics. See `docs/architecture.md`.

- **This crate prints; it doesn't word messages.** A diagnostic's text, code
  and labels are written by the stage that found it (inference messages come
  from `InferError::render` in kestrel-type-infer). Don't add per-error
  formatting here.
- **One bad span must not hide the batch.** `emit_all` renders each diagnostic
  separately. Keep it that way if you change emission.
- Codes and their ranges are listed in `docs/error-codes.md` at the repo root.
