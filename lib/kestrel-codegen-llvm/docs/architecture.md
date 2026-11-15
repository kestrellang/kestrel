# kestrel-codegen-llvm Architecture

The LLVM backend. It turns a monomorphized `MonoModule` into an object
file and links it into an executable. The CLI uses it for -O1 and -O2 (optimized) builds;
`Backend::for_opt` in `src/main.rs` is the one place that choice is made.

## Pipeline Position

```
Source Text → Tokens → CST → AST Build → Name Res → HIR Lower → Type Infer → MIR → Mono → Codegen
                                                                                           ^^^^^^^
                                                                                    this crate (-O1 and -O2 (optimized))
```

The other backend, `kestrel-codegen-cranelift`, has the same module layout. Both
take their type representation and ABI rules from `kestrel-codegen`, so they
can't disagree about how a value looks at the machine level. A change to how
an instruction lowers usually needs the twin change in the other backend.

## Core Types

| Type | Module | Description |
|------|--------|-------------|
| `CodegenOptions` | `lib.rs` | Optimization level, libraries and link arguments, C sources, overflow-check override (on for unoptimized builds by default) |
| `CompilationResult` | `lib.rs` | The object bytes |
| `CodegenCtx` | `context.rs` | Per-module state and the phase driver |
| `CodegenError` | `error.rs` | Backend errors |

Entry points: `compile(module, target, options)` and
`compile_and_link(…, output_path)`.

## Algorithm

`CodegenCtx::compile_all` runs in phases: vtables and statics, then declare
every function, then C bridges, then define every function. A function body
that fails to lower becomes a trap stub instead of failing the whole module.
Addresses are typed LLVM `ptr` values, byte offsets are `getelementptr`, and MIR block arguments become phi incomings. `Switch` lowers to a comparison chain. Object emission goes through the LLVM `TargetMachine`, which also runs the optimization pipeline.

## Module Map

| File | Responsibility |
|------|---------------|
| `context.rs` | Orchestration: vtables, statics, declarations, definitions, trap stubs, object emission |
| `func.rs` | Per-function lowering: blocks, params |
| `inst.rs` | Instruction lowering, the bulk of the backend |
| `terminator.rs` | Branches, switches, returns; MIR block args become block params / phi incomings |
| `imm.rs` | Constants and literals |
| `mem.rs` | Loads, stores, copies, address arithmetic |
| `abi.rs` | Kestrel calling convention, built from `kestrel_codegen::abi` |
| `c_bridge.rs` | Bridges between Kestrel's ABI and the C ABI for `@extern` / `@export` |
| `ty.rs` | The backend's machine scalar type |
| `link.rs` | Object → executable by shelling out to the system linker driver |
| `error.rs` | `CodegenError` |

## Debugging

- `kestrel dump llvm f.ks` prints this backend's IR.
- `KESTREL_DEBUG=llvm-verify` dumps LLVM functions that fail verification (LLVM backend).

## Dependencies

| Crate | Usage |
|-------|-------|
| `kestrel-codegen` | Shared representation, ABI and layout lookups |
| `kestrel-mir` | `MonoModule` |
| `kestrel-hecs`, `kestrel-debug` | `Entity`, `ktrace!` |
| `inkwell` (LLVM 18) | Code generation |
