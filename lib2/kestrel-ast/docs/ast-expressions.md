# AST Expressions

Expressions are stored in an `Arena<AstExpr>` inside `AstBody`, addressed by `ExprId`. They are unresolved — paths are just names, no symbol resolution, no embedded types.

Grouping parentheses are dropped during lowering. For-loops are NOT desugared. Binary operators are flat (no precedence tree at the AST level).

## `AstExpr` Variants

### Literals

| Variant | Syntax | Fields |
|---------|--------|--------|
| `Literal` | `42`, `3.14`, `"hello"`, `true`, `null`, `()` | `kind: AstLiteral` |
| `InterpolatedString` | `"Hello \(name)!"` | `parts: Vec<StringPart>` |

`AstLiteral` variants: `Integer(String)`, `Float(String)`, `String(String)`, `RawString(String)`, `Char(String)`, `Bool(bool)`, `Null`, `Unit`.

Literal values are stored as source text strings (not parsed numbers) to preserve formatting.

### Collections

| Variant | Syntax | Fields |
|---------|--------|--------|
| `Array` | `[1, 2, 3]` | `elements: Vec<ExprId>` |
| `Dictionary` | `["a": 1, "b": 2]` | `entries: Vec<DictEntry>` |
| `Tuple` | `(1, "hello")` | `elements: Vec<ExprId>` |

### Paths & Access

| Variant | Syntax | Fields |
|---------|--------|--------|
| `Path` | `foo`, `Foo.bar`, `Array[Int]` | `segments: Vec<ExprPathSegment>` |
| `MemberAccess` | `expr.member`, `expr.method[T]` | `base: ExprId`, `member: String`, `type_args: Option<Vec<AstType>>` |
| `TupleIndex` | `pair.0` | `base: ExprId`, `index: u32` |
| `ImplicitMember` | `.None`, `.Some(x)` | `member: String`, `arguments: Option<Vec<CallArg>>` |

**Path vs MemberAccess**: Pure identifier chains like `Foo.Bar.baz` produce `Path` with multiple segments. When the base is a computed expression (e.g. `foo().bar`), the lowerer produces a `MemberAccess` with the base expression lowered recursively.

### Operators

| Variant | Syntax | Fields |
|---------|--------|--------|
| `Unary` | `-x`, `not x`, `^x`, `+x` | `op: UnaryOp`, `operand: ExprId` |
| `Postfix` | `x!` | `operand: ExprId`, `op: PostfixOp` |
| `Binary` | `a + b`, `a == b`, `a and b` | `lhs: ExprId`, `op: BinaryOp`, `rhs: ExprId` |
| `Assignment` | `x = 42` | `lhs: ExprId`, `rhs: ExprId` |
| `CompoundAssignment` | `x += 1` | `lhs: ExprId`, `op: CompoundAssignOp`, `rhs: ExprId` |

See [Operator Enums](#operator-enums) below for all variants.

### Calls

| Variant | Syntax | Fields |
|---------|--------|--------|
| `Call` | `foo(x: 1, 2)` | `callee: ExprId`, `arguments: Vec<CallArg>` |

`CallArg` has `label: Option<String>` and `value: ExprId`.

### Control Flow

| Variant | Syntax | Fields |
|---------|--------|--------|
| `If` | `if cond { } else { }` | `conditions: Vec<IfCondition>`, `then_body: AstBlock`, `else_body: Option<ElseBody>` |
| `While` | `while cond { }` | `label: Option<String>`, `condition: ExprId`, `body: AstBlock` |
| `WhileLet` | `while let p = e { }` | `label: Option<String>`, `conditions: Vec<IfCondition>`, `body: AstBlock` |
| `Loop` | `loop { }` | `label: Option<String>`, `body: AstBlock` |
| `For` | `for x in items { }` | `label: Option<String>`, `pattern: PatId`, `iterable: ExprId`, `body: AstBlock` |
| `Break` | `break`, `break label` | `label: Option<String>` |
| `Continue` | `continue`, `continue label` | `label: Option<String>` |
| `Return` | `return`, `return 42` | `value: Option<ExprId>` |
| `Throw` | `throw err` | `value: ExprId` |
| `Try` | `try expr` | `operand: ExprId` |

**Conditions**: `IfCondition` is either `Expr(ExprId)` for plain boolean conditions or `Let { pattern, value }` for pattern-binding conditions. If/while-let/guard-let all use the same type.

**ElseBody**: either `Block(AstBlock)` or `ElseIf(ExprId)` (pointing to a nested `If` expression).

### Closures & Match

| Variant | Syntax | Fields |
|---------|--------|--------|
| `Closure` | `{ (x) in x + 1 }` | `params: Vec<ClosureParam>`, `body: AstBlock` |
| `Match` | `match x { .A => 1, .B => 2 }` | `scrutinee: ExprId`, `arms: Vec<MatchArm>` |

`ClosureParam` has `pattern: PatId` and `ty: Option<AstType>`.

`MatchArm` has `pattern: PatId`, `guard: Option<ExprId>`, and `body: ExprId`.

### Error Recovery

| Variant | Syntax | Fields |
|---------|--------|--------|
| `Error` | (malformed) | (none) |

Produced for unrecognized CST nodes. Allows lowering to continue without panicking.

## Operator Enums

### `UnaryOp`
`Neg` (`-`), `BitNot` (`^`), `LogicalNot` (`not`/`!`), `Pos` (`+`)

### `PostfixOp`
`Unwrap` (`!`)

### `BinaryOp`
Arithmetic: `Add`, `Sub`, `Mul`, `Div`, `Rem`
Bitwise: `BitAnd`, `BitOr`, `BitXor`, `Shl`, `Shr`
Comparison: `Eq`, `Ne`, `Lt`, `Gt`, `Le`, `Ge`
Logical: `And`, `Or`
Other: `Coalesce` (`??`), `RangeInclusive` (`..=`), `RangeExclusive` (`..<`)

### `CompoundAssignOp`
`AddAssign`, `SubAssign`, `MulAssign`, `DivAssign`, `RemAssign`, `BitAndAssign`, `BitOrAssign`, `BitXorAssign`, `ShlAssign`, `ShrAssign`

## Supporting Types

### `StringPart`
Used inside `InterpolatedString`:
- `Literal(String)` — raw text segment
- `Interpolation { expr: ExprId, format: Option<String> }` — `\(expr)` or `\(expr:format)`

### `DictEntry`
`key: ExprId`, `value: ExprId`

### `ExprPathSegment`
`name: String`, `type_args: Option<Vec<AstType>>`, `span: Span`

### `AstBlock`
Nested code block used by control flow expressions. Contains `stmts: Vec<StmtId>` and `tail_expr: Option<ExprId>`.
