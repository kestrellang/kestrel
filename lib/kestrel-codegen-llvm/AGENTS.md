# kestrel-codegen-llvm — agent guide

LLVM code generation backend (inkwell 0.9 / LLVM 18). It started as a port of
`kestrel-codegen-cranelift` — same module layout, same `MonoModule` input, same
public surface (`compile`, `compile_and_link`, `CodegenOptions`,
`CompilationResult`, `CodegenError`). The cranelift module with the same name is
still a useful reference for control flow, BUT the representation now **diverges**:
addresses are real LLVM `ptr` (typed-`ptr`), not cranelift's `i64` (see below).

## Build / setup

- Needs LLVM 18.1 dev libs. `LLVM_SYS_181_PREFIX` must point at them
  (`/opt/homebrew/opt/llvm@18`); it is set in the worktree `.cargo/config.toml`.
- Do **not** add a dependency on `kestrel-codegen-cranelift`. Shared types are
  intentionally duplicated (cf. [[feedback_mir3_independent]] philosophy).

## The representation model — typed `ptr` (do not break this)

Pointer-width scalars are real LLVM **`ptr`** values (`ScalarTy::Ptr`), NOT
`i64`. This is a deliberate divergence from the cranelift backend (`ptr_ty =
I64`): it preserves pointer provenance so LLVM can devirtualize indirect calls,
hoist loads (LICM), and vectorize. Consequences:

- Byte-offset address math is `getelementptr`, via `mem::field_gep` (`inbounds`,
  for compiler-generated within-object offsets) or `mem::raw_gep` (plain, for
  `Op::PtrOffset` user pointer arithmetic that may reach one-past-the-end).
  NEVER `build_int_add` on an address.
- Addresses, aggregate values, and pointer-typed scalars are all `PointerValue`
  in `value_map`. Only int/float scalars differ.
- The ONLY genuine `int<->ptr` conversions are `Op::PtrToAddress` (`ptrtoint`)
  and `Op::PtrFromAddress` (`inttoptr`); `mem::int_to_ptr`/`ptr_to_int` exist
  solely for those (plus the `main` aggregate-return marshalling).
- `ScalarTy::Ptr` is neither int nor float; `is_float` is a positive match (NOT
  `!is_int`), and `bytes()` returns 8 (the backend assumes 64-bit — see
  `TypeCache::new`'s `debug_assert`).
- `Str`/`FuncThick` are `{ ptr@0, <int/ptr>@ptr_size }`: `StrLen` loads the
  length as `I64` (NOT `ptr_scalar`, which is now a `ptr`); the closure fn/env
  slots are both `ptr`.

## Value model

- @owned scalar → the LLVM value IS the scalar (int/float/`ptr`).
- @guaranteed scalar → a `ptr` ADDRESS; `FuncCompiler::resolve_scalar` loads it.
- aggregate (any ownership) → a `ptr` ADDRESS.
- ZST → a null-`ptr` placeholder.
- single-field newtype collapses to its field's repr — so `compile_struct_extract`
  must NOT eagerly `into_pointer_value()` the operand (a newtype's value IS the
  field, possibly a non-pointer scalar).

Layouts (size/align/field offsets, enum discriminant width, variant layouts)
come from the MIR layout pass (`type_info.layout`) — the single source of truth.
**Never recompute layout here.** The single-field newtype collapse in
`ty::classify_named` must delegate to the field's repr (a `Float64` is `f64`,
not `i64`) — see [[per_instantiation_copy_semantics]].

## ABI (manual)

No LLVM `sret`/`byval`/`noalias` attributes (a deliberate follow-up — adding
them is the next provenance tier, but it changes the platform ABI so it's not
part of the typed-`ptr` base). Aggregates pass by pointer: a leading `ptr` sret
param + manual `mem::copy_aggregate`. Param/return classification lives in
`abi.rs` (`param_pass_mode`/`return_mode`); the sret/ByRef/aggregate param type
is `ptr_scalar.llvm` (now `ptr`), so the bodies are unchanged from the i64 era.
Callers and callees both derive signatures from `ptr_scalar`, so they stay in
lockstep; extern "C" pointer params are `ptr` (what C wants), integer params
(`size_t`-like) stay `I64`.

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
