# kestrel-compiler

`Compiler`: owns the world, loads files, runs the front end, MIR and codegen.
See `docs/architecture.md`.

- **The std is told apart from user code by the `StdlibSource` marker** set in
  `set_std_source`, never by module name.
- **`stdlib_path.rs` is the only std locator**, used by the CLI, the LSP and
  the test suite. Its precedence order is load-bearing; don't add another
  lookup.
- **The post-mono pass order is in `monomorphize_mir_until`.** The order is
  load-bearing (the mono flow check must run before `expand`). Adding or
  moving a pass updates the places listed in `lib/kestrel-mir/AGENTS.md`.
- **Lowering errors gate codegen.** MIR lowering can accumulate hard errors
  (E503) and still return a complete module; `ensure_no_lowering_errors` stops
  a binary being written in that case (#152). Keep that gate on every codegen
  path.
- **A caller of `lower_to_mir_stage` must flush the accumulator.** It returns
  verify errors but lowering also files its own diagnostics (F18).
