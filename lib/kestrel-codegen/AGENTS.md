# kestrel-codegen

The representation and ABI rules both backends share. See `docs/architecture.md`.

- **A rule about what a value looks like at machine level goes here, once.**
  Scalar vs aggregate, pass and return modes, the C ABI and field offsets.
  If Cranelift (`-O0`) and LLVM (`-O1`/`-O2`) each had their own, a program
  could behave differently at different optimization levels.
- **Layouts come from mono** (`type_info.layout`). Read them through
  `layout.rs`; never recompute a size or offset here or in a backend.
- **A change here changes both backends.** Check a program built with `-O0`
  (Cranelift) and with `-O2` (LLVM, the default).
- Matches over `MirTy` stay exhaustive, so a new type variant forces a decision.
- Backend-specific rules are in each backend's AGENTS.md.
