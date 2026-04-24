# kestrel-hir Architecture

HIR (High-level Intermediate Representation) data types for the Kestrel compiler. A desugared, partially-resolved representation that bridges the AST and type inference.

## Pipeline Position

```
Source → Tokens → CST (rowan) → AST entities (ECS) → Name Resolution → HIR → Type Inference → MIR → Codegen
                                                                         ^^^
```

HIR lowering takes the AST and name resolution results and produces `HirBody` — a flat, desugared representation of each function/getter/setter body.

## What Changes from AST to HIR

| Transformation | AST | HIR |
|---------------|-----|-----|
| Operators | `Binary { lhs, op: Add, rhs }` | `ProtocolCall { protocol: Addable, method: "add" }` |
| For-loops | `For { pattern, iterable, body }` | `Loop` + `ProtocolCall` (iterator protocol) |
| While-loops | `While { condition, body }` | `Loop` + `If` + `Break` |
| Try/throw | `Try { operand }`, `Throw { value }` | `ProtocolCall` (Tryable/Throwable) |
| String interpolation | `InterpolatedString { parts }` | Series of `ProtocolCall` (Addable) |
| Guard-let | `GuardLet { pattern, value, else }` | `If` + diverging block |
| Literals | Source text (`"42"`) | Parsed values (`i64(42)`) |
| Type sugar | `Int?`, `[Int]`, `[K: V]` | `Named(Optional, [Named(Int)])` etc. |
| Scope names | `Path { segments }` | `Local(id)` or `Def(entity)` |
| Type-dependent names | `MemberAccess { member }` | `Field { name }` / `MethodCall { method }` (strings) |

## Two-Level Resolution

Names that can be resolved from scope alone (imports, locals, top-level definitions) are resolved to `Entity` or `LocalId` during HIR lowering. Names that depend on the receiver's type (field access, method calls, implicit enum members) remain as strings for type inference to resolve later.

**Resolved at HIR lowering** (scope-dependent):
- `Local(LocalId)` — local variables, parameters, pattern bindings
- `Def(Entity)` — functions, structs, enums, enum cases, protocols, type aliases
- `ProtocolCall { protocol: Entity }` — protocol entity for desugared operators

**Deferred to type inference** (type-dependent):
- `Field { name: String }` — struct field access
- `MethodCall { method: String }` — method calls
- `ImplicitMember { name: String }` — `.Case` enum shorthand
- `ImplicitVariant { name: String }` — `.Case` in patterns

## `HirBody`

Top-level container for a function/getter/setter body after HIR lowering.

| Field | Type | Description |
|-------|------|-------------|
| `exprs` | `Arena<HirExpr>` | All expressions in the body |
| `pats` | `Arena<HirPat>` | All patterns in the body |
| `stmts` | `Arena<HirStmt>` | All statements in the body |
| `locals` | `Arena<Local>` | Local variable slots |
| `params` | `Vec<LocalId>` | Function parameters in declaration order |
| `statements` | `Vec<HirStmtId>` | Top-level statements |
| `tail_expr` | `Option<HirExprId>` | Trailing expression (block value) |

All nodes are stored in flat arenas and addressed by typed indices (`HirExprId`, `HirPatId`, `HirStmtId`, `LocalId`). No heap-allocated trees — just index references between arena slots.

## Module Map

| File | Responsibility |
|------|---------------|
| `lib.rs` | Crate root, re-exports public API |
| `body.rs` | `HirBody`, `HirExpr` (19 variants), `HirStmt` (3), `HirPat` (10), `HirLiteral`, operator desugaring tables |
| `res.rs` | `Res` (name resolution result), `Local`, `LocalId` |
| `ty.rs` | `HirTy` (6 variants — resolved type representations) |

## Dependencies

| Crate | Usage |
|-------|-------|
| `kestrel-ast` | `Arena<T>`, `Idx<T>` (arena storage), `BinaryOp`, `UnaryOp`, `CompoundAssignOp` (operator enums) |
| `kestrel-span2` | `Span` (source location tracking) |
| `kestrel-hecs` | `Entity` (ECS handles for resolved definitions) |
