# kestrel-target Architecture

The compilation target (triple, pointer width, `@platform` OS), defined once for the whole pipeline.

## Pipeline Position

```
Source Text → Tokens → CST → AST Build → Name Res → HIR Lower → Type Infer → MIR → Mono → Codegen
                              ^^^^^^^^^                                       ^^^^^^^^^^^^^^^^^^^
                              @platform                                       layout, backends
```

A leaf crate: the driver builds one `TargetConfig` from `--target` (or the host) and every stage reads it.

## Core Types

| Type | Module | Description |
|------|--------|-------------|
| `TargetConfig` | `lib.rs` | Target triple, pointer width, and `@platform` OS. `host()` / `from_triple()`. |
| `Os` | `lib.rs` | `@platform` OS cases (`Darwin`, `Linux`). |

## Who reads what

| Reader | Uses |
|--------|------|
| `kestrel-ast-builder` | `os()` to drop `@platform`-excluded declarations |
| `kestrel-mir` (layout, mono) | `pointer_size()` for pointer, ref, `str` and closure sizes |
| `kestrel-codegen-cranelift` / `-llvm` | `triple` and `pointer_size()` for ISA, data layout, linking |

## Key Design Decisions

- **One definition.** MIR layout and codegen previously each had their own pointer width (MIR's was always 8), so a 32-bit target would have laid types out at one size and emitted code at another. Add new target facts here, not in a stage.
- **Unknown OS ⇒ no filtering.** A triple whose OS has no `@platform` case gives `os() == None`, which keeps every declaration.

## Dependencies

| Crate | Usage |
|-------|-------|
| `target-lexicon` | Triple parsing and pointer width |
