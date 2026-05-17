# HIR Expressions

Expressions are stored in an `Arena<HirExpr>` inside `HirBody`, addressed by `HirExprId`. All syntactic sugar (operators, for-loops, while-loops, try/throw, string interpolation) has been desugared. 19 variants total.

## `HirExpr` Variants

### Values

| Variant | Syntax | Fields |
|---------|--------|--------|
| `Literal` | `42`, `3.14`, `"hello"`, `true`, `null` | `value: HirLiteral` |
| `Tuple` | `(1, "hello")` | `elements: Vec<HirExprId>` |
| `Array` | `[1, 2, 3]` | `elements: Vec<HirExprId>` |
| `Dict` | `["a": 1, "b": 2]` | `entries: Vec<HirDictEntry>` |
| `Closure` | `{ (x) in x + 1 }` | `params: Vec<HirClosureParam>`, `body: HirBlock` |

### Resolved References

| Variant | Syntax | Fields |
|---------|--------|--------|
| `Local` | `x` (local variable) | `LocalId`, `Span` |
| `Def` | `foo` (function/type/enum case) | `Entity`, `Span` |

Resolved during HIR lowering via name resolution. `Local` for stack-allocated variables; `Def` for top-level definitions accessible by scope.

### Member Access

| Variant | Syntax | Fields |
|---------|--------|--------|
| `Field` | `point.x` | `base: HirExprId`, `name: String` |
| `TupleIndex` | `pair.0` | `base: HirExprId`, `index: u32` |
| `ImplicitMember` | `.None`, `.Some(x)` | `name: String`, `args: Option<Vec<HirCallArg>>` |

`Field` and `ImplicitMember` store names as strings — resolved by type inference based on the receiver/expected type.

### Calls

| Variant | Syntax | Fields |
|---------|--------|--------|
| `Call` | `foo(x)`, `Point(x: 1, y: 2)` | `callee: HirExprId`, `args: Vec<HirCallArg>` |
| `MethodCall` | `x.foo()`, `x.map[Int](...)` | `receiver: HirExprId`, `method: String`, `type_args: Option<Vec<HirTy>>`, `args: Vec<HirCallArg>` |
| `ProtocolCall` | *(from desugared operators)* | `receiver: HirExprId`, `protocol: Entity`, `method: String`, `type_args: Option<Vec<HirTy>>`, `args: Vec<HirCallArg>` |

`ProtocolCall` is produced by operator desugaring — never appears in user-written code directly. The `protocol` entity is resolved by name resolution; type inference generates a conformance constraint ensuring the receiver conforms to the protocol. See [desugaring.md](desugaring.md).

### Control Flow

| Variant | Syntax | Fields |
|---------|--------|--------|
| `If` | `if cond { } else { }` | `condition: HirExprId`, `then_body: HirBlock`, `else_body: Option<HirBlock>` |
| `Loop` | `loop { }` | `label: Option<String>`, `body: HirBlock` |
| `Match` | `match x { .A => 1 }` | `scrutinee: HirExprId`, `arms: Vec<HirMatchArm>` |
| `Break` | `break`, `break label` | `label: Option<String>` |
| `Continue` | `continue`, `continue label` | `label: Option<String>` |
| `Return` | `return`, `return 42` | `value: Option<HirExprId>` |

`While` and `For` loops are desugared into `Loop` + `If` + `Break` (and protocol calls for iterators).

### Other

| Variant | Syntax | Fields |
|---------|--------|--------|
| `Assign` | `x = 42` | `target: HirExprId`, `value: HirExprId` |
| `Error` | *(malformed)* | *(none)* |

## `HirBlock`

Nested code block used by `If`, `Loop`, `Closure`, and desugared constructs.

| Field | Type | Description |
|-------|------|-------------|
| `stmts` | `Vec<HirStmtId>` | Statements in the block |
| `tail_expr` | `Option<HirExprId>` | Trailing expression (block value) |

## `HirLiteral`

Parsed literal values. Unlike `AstLiteral` which stores source text strings, HIR literals are parsed into concrete types.

| Variant | Rust Type | Example |
|---------|-----------|---------|
| `Integer` | `i64` | `42` |
| `Float` | `f64` | `3.14` |
| `String` | `String` | `"hello"` |
| `Char` | `u32` | `'A'` (Unicode scalar, `<= 0x10FFFF`) |
| `Bool` | `bool` | `true` |
| `Null` | — | `null` |

Implements `PartialEq` for literal equality checking (used in pattern matching).

## Supporting Types

### `HirCallArg`

| Field | Type |
|-------|------|
| `label` | `Option<String>` |
| `value` | `HirExprId` |

### `HirDictEntry`

| Field | Type |
|-------|------|
| `key` | `HirExprId` |
| `value` | `HirExprId` |

### `HirMatchArm`

| Field | Type |
|-------|------|
| `pattern` | `HirPatId` |
| `guard` | `Option<HirExprId>` |
| `body` | `HirExprId` |

### `HirClosureParam`

| Field | Type |
|-------|------|
| `local` | `LocalId` |
| `ty` | `Option<HirTy>` |

## HIR Statements

3 variants. `GuardLet` is desugared into `If` + diverging block before reaching HIR.

| Variant | Syntax | Fields |
|---------|--------|--------|
| `Let` | `let x: Int = 42` | `local: LocalId`, `ty: Option<HirTy>`, `value: Option<HirExprId>` |
| `Expr` | `foo()` | `expr: HirExprId` |
| `Deinit` | `deinit name` | `name: String` |

## HIR Patterns

10 variants. `At` and `Rest` patterns from the AST are absorbed during lowering.

| Variant | Syntax | Fields |
|---------|--------|--------|
| `Wildcard` | `_` | — |
| `Binding` | `x` | `local: LocalId` |
| `Tuple` | `(a, b)` | `elements: Vec<HirPatId>` |
| `Literal` | `42`, `"hi"` | `value: HirLiteral` |
| `Range` | `1..=10`, `1..<10` | `start: Option<HirLiteral>`, `end: Option<HirLiteral>`, `inclusive: bool` |
| `Variant` | `Optional.Some(x)` | `entity: Entity`, `args: Vec<HirPatArg>` |
| `ImplicitVariant` | `.Some(x)` | `name: String`, `args: Vec<HirPatArg>` |
| `Struct` | `Point { x, y, .. }` | `entity: Entity`, `fields: Vec<HirStructPatField>`, `has_rest: bool` |
| `Or` | `1 \| 2 \| 3` | `alternatives: Vec<HirPatId>` |
| `Error` | *(malformed)* | — |

`Variant` is fully resolved by name resolution. `ImplicitVariant` stores the name as a string for type inference to resolve based on the scrutinee type.

### `HirPatArg`

| Field | Type |
|-------|------|
| `label` | `Option<String>` |
| `pattern` | `HirPatId` |

### `HirStructPatField`

| Field | Type |
|-------|------|
| `field_name` | `String` |
| `pattern` | `Option<HirPatId>` |
