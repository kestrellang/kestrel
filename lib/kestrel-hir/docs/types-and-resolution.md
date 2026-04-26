# Types and Name Resolution

## `HirTy`

Resolved type representations. All syntactic sugar is expanded before reaching HIR — `Optional`, `Array`, `Dictionary`, `Result` are just `Named` with the appropriate entity and type arguments. 6 variants (down from AST's 10).

| Variant | Syntax | Fields |
|---------|--------|--------|
| `Named` | `Int64`, `Array[Int]`, `Optional[String]` | `entity: Entity`, `args: Vec<HirTy>` |
| `Tuple` | `(Int, String)` | `Vec<HirTy>` |
| `Function` | `(Int) -> String` | `params: Vec<HirTy>`, `ret: Box<HirTy>` |
| `Param` | `T` (type parameter) | `Entity` |
| `Infer` | `_` or omitted | — |
| `Error` | *(malformed)* | — |

### Sugar Resolution Examples

| Source | HIR |
|--------|-----|
| `Int?` | `Named(Optional, [Named(Int)])` |
| `[Int]` | `Named(Array, [Named(Int)])` |
| `[String: Int]` | `Named(Dictionary, [Named(String), Named(Int)])` |
| `Int throws Error` | `Named(Result, [Named(Int), Named(Error)])` |
| `(Int, String)` | `Tuple([Named(Int), Named(String)])` |
| `(Int) -> Bool` | `Function(params: [Named(Int)], ret: Named(Bool))` |

## `Res`

What a name resolves to. Produced by name resolution, consumed by HIR lowering.

| Variant | Description |
|---------|-------------|
| `Local(LocalId)` | Local variable (stack slot) |
| `Def(Entity)` | ECS entity: function, struct, enum, enum case, field, protocol, type alias, type parameter |
| `SelfValue` | `self` keyword |
| `Err` | Unresolved name (error recovery) |

## `Local` and `LocalId`

`LocalId` is a typed index (`Idx<Local>`) into the `locals` arena in `HirBody`.

| Field | Type | Description |
|-------|------|-------------|
| `name` | `String` | Variable name |
| `is_mut` | `bool` | `true` for `var`, `false` for `let` |
| `span` | `Span` | Declaration site |

Locals are allocated during HIR lowering for:
- `let`/`var` bindings
- Function parameters
- Pattern bindings (match arms, if-let)
- Synthetic variables (desugared for-loop iterators, etc.)
