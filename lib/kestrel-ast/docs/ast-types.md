# AST Types

Type annotations extracted from the CST during the build phase. Stored as data (not entities) in `TypeAnnotation` / `Callable` components; a type written inside a body (`let x: T`, `f[T]`, `{ (x: T) in … }`) is read from the CST by body lowering into the same `AstType`.

All types carry a `Span` for error reporting.

## `AstType`

| Variant | Syntax | Fields |
|---------|--------|--------|
| `Named` | `Int64`, `Array[Int]`, `std.collections.Map[K, V]` | `segments: Vec<PathSegment>` |
| `Tuple` | `(Int, String)` | `Vec<AstType>` |
| `Function` | `(Int) -> String` | `params: Vec<AstType>`, `return_type: Box<AstType>` |
| `Array` | `[Int]` | `Box<AstType>` |
| `Dictionary` | `[String: Int]` | key: `Box<AstType>`, value: `Box<AstType>` |
| `Optional` | `Int?` | `Box<AstType>` |
| `Result` | `Int throws Error` | `ok: Box<AstType>`, `err: Box<AstType>` |
| `Unit` | `()` | (none) |
| `Never` | `Never` | (none) |
| `Inferred` | `_` | (none) |

## `PathSegment`

A single segment in a qualified type path. Each segment has a name and optional type arguments.

```
Array[Int].Iterator
^^^^^^^^^  ^^^^^^^^
segment 1  segment 2
```

| Field | Type | Description |
|-------|------|-------------|
| `name` | `String` | Segment identifier |
| `type_args` | `Vec<AstType>` | Type arguments (empty if none) |
| `span` | `Span` | Source location |

## Where Types Appear

- `TypeAnnotation` component (field types, return types, alias targets)
- Inside bodies there is no stored `AstType`: `kestrel-hir-lower` converts
  type syntax it meets in the CST (`let` annotations, closure parameter and
  return types, explicit type arguments on paths and members, casts) with
  `lower_type` and resolves the result to `HirTy` straight away.
