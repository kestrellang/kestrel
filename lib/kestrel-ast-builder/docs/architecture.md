# AST Builder Architecture

## Pipeline Position

```
Source Text → Tokens → CST (rowan) → AST Builder → ECS World → Queries
              ↑lex      ↑parse       ↑build_declarations    ↑name resolution, type checking
```

The AST builder sits between parsing and semantic analysis. It walks the rowan CST and creates **declaration entities** with **components** in the ECS world. Expressions are not processed — they remain as CST subtrees stored in `Valued` components for later phases.

## Mutation Phase

`build_declarations` runs during the mutation phase (`&mut World`), not as a query. This is intentional:

- It creates entities (`world.spawn()`) and sets components (`world.set()`)
- It establishes parent-child relationships (`world.set_parent()`)
- The ECS world is mutable during this phase

Subsequent compilation phases (name resolution, type checking) run as **queries** in the read phase, accessing the entities and components created here.

## Algorithm

The builder uses an **iterative stack** rather than recursion:

1. Extract the module path from `ModuleDeclaration` (if present)
2. Find-or-create the module hierarchy under the root entity
3. Push top-level children onto the stack with the module as parent
4. Pop nodes, dispatch by `SyntaxKind` to `build_*` functions
5. Container types (struct/enum/protocol/extension) push their body children back onto the stack

## Module Hierarchy

Modules are shared across files. When two files declare `module Shared`, they reuse the same module entity:

```
root
└── Shared (module entity — no FileId)
    ├── StructA (from file1.ks — has FileId(file1))
    └── StructB (from file2.ks — has FileId(file2))
```

The find-or-create algorithm walks the dotted path left-to-right. For each segment, it scans `children_of(parent)` for an existing `NodeKind::Module` + `Name` match. Creates if not found.

Module entities have **no FileId** because they span multiple files.

## Component Design

Components describe **capabilities** — what an entity CAN DO. They are orthogonal and composable:

- `Typed` — this entity IS a type (can appear in type positions)
- `Callable` — has a parameter list, can be invoked
- `Gettable` / `Settable` — can be read/written as a value
- `Valued` — has a body or initializer expression
- `Static` — accessed through a type, not an instance

This is intentionally flat rather than hierarchical. A subscript is `Callable + Gettable + Settable + Subscript`, not a special "subscript type". Downstream queries can pattern-match on capability combinations.

## Incrementality

Two layers:

1. **File-level skip**: If a file's source hasn't changed, skip `build_declarations` entirely
2. **Component fingerprint backdating**: When a component's value hasn't changed, backdate its revision so downstream queries don't re-execute

Both are handled at the `Compiler` level, not within the builder itself.
