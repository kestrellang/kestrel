# kestrel-codegen-llvm — agent guide

LLVM code generation backend (inkwell 0.9 / LLVM 18). It is a **faithful port of
`kestrel-codegen-cranelift`** — same module layout, same `MonoModule` input, same
public surface (`compile`, `compile_and_link`, `CodegenOptions`,
`CompilationResult`, `CodegenError`). When in doubt, read the cranelift module
with the same name; the lowering decisions are meant to match it exactly.

## Build / setup

- Needs LLVM 18.1 dev libs. `LLVM_SYS_181_PREFIX` must point at them
  (`/opt/homebrew/opt/llvm@18`); it is set in the worktree `.cargo/config.toml`.
- Do **not** add a dependency on `kestrel-codegen-cranelift`. Shared types are
  intentionally duplicated (cf. [[feedback_mir3_independent]] philosophy).

## The representation model — "Option A" (do not break this)

Pointer-width scalars are LLVM **`i64`/`i32`**, NOT LLVM `ptr` (mirrors
cranelift's `ptr_ty = I64`). LLVM `ptr` materialises ONLY at memory-access and
indirect-call sites, via `inttoptr`, centralized in `mem.rs`. Consequences:

- Byte-offset address math is `build_int_add` (see `inst::offset_addr`), never GEP.
- Addresses, aggregate values, and pointer-typed scalars are all `IntValue` of
  pointer width in `value_map`. Only int/float scalars differ.
- New memory access? Route it through `mem.rs` so the `inttoptr` stays in one place.

## Value model (identical to cranelift `func.rs`)

- @owned scalar → the LLVM value IS the scalar.
- @guaranteed scalar → an i64 ADDRESS; `FuncCompiler::resolve_scalar` loads it.
- aggregate (any ownership) → an i64 ADDRESS.
- ZST → placeholder i64 `0`.

Layouts (size/align/field offsets, enum discriminant width, variant layouts)
come from the MIR layout pass (`type_info.layout`) — the single source of truth.
**Never recompute layout here.** The single-field newtype collapse in
`ty::classify_named` must delegate to the field's repr (a `Float64` is `f64`,
not `i64`) — see [[per_instantiation_copy_semantics]].

## ABI (manual, like cranelift)

No LLVM `sret`/`byval` attributes. Aggregates pass by pointer: a leading i64
sret param + manual `mem::copy_aggregate`. Param/return classification lives in
`abi.rs` (`param_pass_mode`/`return_mode`) — keep it byte-identical to the
cranelift `abi.rs`, or call ABIs will disagree with extern C and across calls.

## CFG lowering invariants

- MIR block params → **LLVM phi nodes** (`block_phis`, created in `func.rs`).
- **CRITICAL:** a phi may have only one entry per predecessor block. When a
  branch's two edges target the SAME block (switch last-arm with no distinct
  wildcard, or `then == else`), emit an **unconditional** branch and add the phi
  args **once**. Adding them twice = "PHI node has multiple entries for the same
  basic block" verify failure. Any new branching terminator must honor this.
- `switch` is a comparison chain (matches cranelift), not an LLVM `switch`.
- Allocas are hoisted to the entry block (`FuncCompiler::alloca`) so they are
  fixed stack slots, not re-run per loop iteration.

## Borrow discipline

The `Builder` is **threaded as a separate `&Builder` argument**, never stored in
`CodegenCtx`, so it never aliases the `&mut CodegenCtx` borrow. `CodegenCtx.cx`
(`&'ctx Context`) and `CodegenCtx.module` (`&'ctx MonoModule`) are Copy
references that outlive the context, so `func`/`body` borrows are independent of
the `&mut ctx` borrow (copy the ref into a local first).

## Fault tolerance

Each function body is built inside `catch_unwind` and then `verify()`ed; failure
→ `reset_to_trap_stub` (an `llvm.trap` + `unreachable`). One bad function must
never sink the whole module. Categorized warnings print to stderr;
`KESTREL_VERBOSE_CODEGEN=1` prints per-function LLVM verify errors.

## inkwell 0.9 gotchas

- Every `build_*` returns `Result<_, BuilderError>`.
- Opaque pointers: `build_load`/`build_gep`/`build_struct_gep` take the **pointee
  type** explicitly. Pass a concrete type (`IntType`/`BasicTypeEnum`), not
  `.into()` into a generic slot (ambiguous).
- `CallSiteValue::try_as_basic_value()` returns `ValueKind`, use `.basic()`
  (NOT `.left()` — that's the older `Either` API).
- `print_to_string` on a `FunctionValue` needs `use inkwell::values::AnyValue`.

## When adding a new MIR variant

A new `InstKind` / `Op` / `TerminatorKind` / `ImmediateKind` variant must be
handled in BOTH backends. Update the matching module here AND in
`kestrel-codegen-cranelift`, or this backend's exhaustive `match` won't compile
(good) / will emit `CodegenError::Unsupported` (logged, trap-stubbed).

## Testing

Validate via the full suite with `KESTREL_BACKEND=llvm` (the test runner reads
it). The triage build hash excludes env vars, so to compare against cranelift,
bump the hash (edit a test-suite source comment) between runs. Goal: identical
failure set to cranelift. Last validated 3037/17, identical — see [[llvm_backend]].
