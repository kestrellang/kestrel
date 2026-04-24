# Kestrel MIR Architecture

The MIR (Mid-level Intermediate Representation) is a flat, explicit representation of
Kestrel programs suitable for analysis, optimization, and codegen. It sits between the
typed AST (ECS) and Cranelift IR.

## Design Principles

1. **Separate, self-describing artifact** — the MIR is a standalone data structure that
   can be printed and understood without the ECS. Entity references carry resolved names
   for display.
2. **Flat namespace** — all items are fully qualified, no nesting.
3. **Explicit** — self types, generics, calling conventions, receiver modes all visible.
4. **Place/Value distinction** — places are memory locations, values are computed results.
5. **No SSA** — places can be reassigned (like Rust MIR).
6. **Generic** — monomorphization happens at codegen time, not in the MIR.
7. **Deterministic** — all maps that appear in output use ordered containers (IndexMap/BTreeMap).

## Why Separate From the ECS?

The ECS scatters information across entities and components. To understand what codegen
sees for a single function, you'd need to chase entities across components, resolve type
params, and look up conformances. The MIR gives you the whole program in one linear read,
fully resolved, no indirection.

This is not just a debugging convenience — it's a comprehension tool. "Show me exactly
what the compiler thinks this program is" with zero abstraction layers to peel back.

### What the MIR duplicates (and why)

Struct/enum/protocol definitions already exist in the ECS. The MIR duplicates them so
that:
- The MIR dump is complete without the ECS
- Codegen depends only on the MIR crate, not the full compiler
- The MIR can be serialized, diffed, or snapshot-tested independently

The duplication is bounded — item shapes are stable and change rarely.

### What the MIR does NOT duplicate

- Name resolution, scope rules, import resolution — these are ECS concerns
- Type inference state, constraints, solver data — fully resolved before MIR
- Source text, parse trees, syntax nodes — MIR carries spans, not text

## Pipeline

```
Semantic Tree (ECS)
    |
    v
Lower (syntax-directed, no deinit logic)
    |
    v
Deinit pass (insert deinit/deinit-if from liveness analysis)
    |
    v
Thunk pass (generate and deduplicate thunks globally)
    |
    v
Layout pass (compute struct sizes, field offsets, drop order)
    |
    v
MirModule (complete, generic, dumpable)
    |
    v
Codegen
  |-- Collection (BFS -> instantiation work list)
  '-- Emit (for each instantiation: read generic MIR + substitution -> Cranelift IR)
```

Each phase is a function from one type to another. Each intermediate is dumpable and
independently testable. No mixed concerns.

### Phase details

**Lower**: Pure syntax-directed translation from the typed AST. No scope tracking, no
deinit logic, no thunk generation. Emits Move/Copy/Ref/RefMut as-is. Produces a raw
MirModule with function bodies but no cleanup code.

**Deinit pass**: Analyzes liveness across the control flow graph. Inserts `Deinit` at
last use of non-copyable locals, `DeinitIf` at branch merge points where a value may
or may not have been moved. This is a well-defined graph algorithm, not interleaved
state tracking.

**Thunk pass**: Scans for `ApplyPartial` references, generates thunk functions that
bridge thin->thick calling conventions, deduplicates globally. All thunks are visible
in the MIR dump.

**Layout pass**: Computes struct sizes, field offsets, alignment, and drop order for
each struct/enum. Results are stored on the struct/enum defs. Codegen reads layouts
from the MIR rather than computing them independently.

**Codegen**: Reads the generic MirModule. Collection phase does BFS to discover all
concrete instantiations. Emit phase compiles each instantiation by reading the generic
MIR body with a substitution map — the substituted MIR never exists as a data structure.
