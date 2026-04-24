# AST Statements

Statements are stored in an `Arena<AstStmt>` inside `AstBody`, addressed by `StmtId`.

An `AstBody` holds a top-level `statements: Vec<StmtId>` list plus an optional `tail_expr: Option<ExprId>` for the trailing expression that produces the block's value.

## `AstStmt` Variants

### `Let`

Variable declaration: `let x = 1;` or `var x: Int64 = 42;`

| Field | Type | Description |
|-------|------|-------------|
| `is_mut` | `bool` | `true` for `var`, `false` for `let` |
| `pattern` | `PatId` | The binding pattern (can be destructuring) |
| `ty` | `Option<AstType>` | Explicit type annotation, if present |
| `value` | `Option<ExprId>` | Initializer expression, if present |
| `span` | `Span` | Source location |

Examples:
```
let x = 42;                  // is_mut=false, pattern=Binding("x"), ty=None, value=Some(42)
var name: String = "hello";  // is_mut=true,  pattern=Binding("name"), ty=Some(String), value=Some("hello")
let (a, b) = pair;           // is_mut=false, pattern=Tuple([Binding("a"), Binding("b")]), value=Some(pair)
```

### `Expr`

Expression statement: any expression followed by a semicolon.

| Field | Type | Description |
|-------|------|-------------|
| `expr` | `ExprId` | The expression |
| `span` | `Span` | Source location |

Statement-like expressions (if/while/loop/for/match) that appear in statement position are wrapped in `Expr` statements.

### `GuardLet`

Guard-let statement: `guard let pattern = expr else { block }`

| Field | Type | Description |
|-------|------|-------------|
| `conditions` | `Vec<IfCondition>` | One or more let-binding conditions |
| `else_body` | `AstBlock` | The else block (must diverge: return/break/continue/throw) |
| `span` | `Span` | Source location |

Conditions use the same `IfCondition` enum as if-let expressions. Multiple conditions can be chained with commas:
```
guard let x = optX, let y = optY else { return; }
```

### `Deinit`

Explicit destructor call: `deinit name;`

| Field | Type | Description |
|-------|------|-------------|
| `name` | `String` | Variable being deinitialized |
| `span` | `Span` | Source location |

## Tail Expressions

A code block's value comes from a trailing expression without a semicolon:

```
func square(x: Int64) -> Int64 {
    x * x       // <-- tail expression, becomes the return value
}
```

In `AstBody`, this is stored as `tail_expr: Some(expr_id)` rather than in the `statements` list.

If a block ends with a semicolon-terminated expression, that expression is a statement (in `statements`), and `tail_expr` is `None`.
