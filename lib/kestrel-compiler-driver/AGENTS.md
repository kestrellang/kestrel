# kestrel-compiler-driver

Whole-world inference and analysis scans and terminal output, for the CLI and
the test suite. See `docs/architecture.md`.

- **Adding an `InferError` variant needs an arm in both `error_variant_name`
  and `format_error` here.** The full checklist is in
  `lib/kestrel-type-infer/AGENTS.md`.
- **Whole-program checks are gated on `analyze_all(is_executable)`**, such as
  the missing-`@main` error (E618). A library build must not report them.
- **`infer_all` catches panics and keeps going.** Any push/pop side state a
  body can touch must be guarded with `kestrel_hecs::guard::OnDrop`, or a
  panic in one body corrupts the rest.
- Keep terminal output here, not in `kestrel-compiler`, so embedders such as
  the LSP don't have it imposed.
