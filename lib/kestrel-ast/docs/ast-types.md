# AST Types

Type annotations extracted from the CST during the build phase. Stored as data (not entities) in `TypeAnnotation` components and embedded in `AstBody` nodes where type references appear.

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
- `AstStmt::Let { ty }` (variable type annotations)
- `ClosureParam { ty }` (closure parameter types)
- `ExprPathSegment { type_args }` (expression-position type arguments)
- `MemberAccess { type_args }` (member access type arguments)
