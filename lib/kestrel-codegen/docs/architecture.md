# kestrel-codegen Architecture

Backend-agnostic code generation rules shared by the Cranelift and LLVM
backends: how a MIR type is represented, how it is passed and returned, and
where a field lives.

## Pipeline Position

```
Source Text → Tokens → CST → AST Build → Name Res → HIR Lower → Type Infer → MIR → Mono → Codegen
                                                                                           ^^^^^^^
                                                                         shared rules → cranelift (-O0)
                                                                                      → llvm (-O1/-O2)
```

## Why it exists

`-O0` builds go through Cranelift and `-O1`/`-O2` through LLVM. If each backend
had its own copy of the representation and ABI rules, the two could disagree
about the same type, and a program would behave differently at different
optimization levels. Everything that decides *what* a value looks like at the
machine level is written once here, generic over the backend's scalar type.
Each backend only maps the result onto its own IR.

## Core Types

| Type | Module | Description |
|------|--------|-------------|
| `TypeRepr` | `repr.rs` | A MIR type as a machine scalar (held in an SSA value) or an aggregate (held by pointer) |
| `ScalarModel` | `repr.rs` | The trait a backend implements to name its scalar types |
| `TypeCache` | `repr.rs` | Memoized `TypeRepr` per type |
| `PassMode` / `ReturnMode` | `abi.rs` | Kestrel's own ABI: scalars by value, aggregates by pointer, aggregate returns through a leading sret pointer, ZSTs take no slot |
| `CSig` / `CArg` / `CRet` | `c_abi.rs` | The C ABI for `@extern` imports, `@export` definitions and C function values: small structs split into registers, large ones copied or passed by pointer, per target |

## Module Map

| File | Responsibility |
|------|---------------|
| `repr.rs` | MIR type → scalar or memory. Layouts come from mono, never recomputed here |
| `abi.rs` | Kestrel calling convention |
| `c_abi.rs` | C calling convention, per target |
| `layout.rs` | Field, tuple-element and discriminant lookups into the mono module's layouts |

## Dependencies

| Crate | Usage |
|-------|-------|
| `kestrel-mir` | `MonoModule`, `TyId`, layouts |
| `kestrel-target` | `TargetConfig` (re-exported) |
| `kestrel-hecs` | `Entity` |
| `kestrel-debug` | `ktrace!` |
