# HIR Lowering: `kestrel-hir-lower`

## Pipeline Position

```
CST → AST Builder (mutation) → ECS World
                                   ↓
                             Name Resolution (queries)
                                   ↓
                             HIR Lowering (this crate) ← LowerBody query
                                   ↓
                             Type Inference (InferBody query)
```

HIR lowering converts `AstBody` (arena-based, unresolved AST) into `HirBody`
(partially-resolved HIR). It sits between name resolution and type inference,
consuming both:

- **From AST builder**: `Body(AstBody)` and `Callable` components on entities
- **From name resolution**: `ResolveValuePath`, `ResolveTypePath`, `ResolveBuiltin` queries (called lazily during lowering)
- **Produces**: `HirBody` consumed by `kestrel-type-infer`'s `InferBody` query

## What This Crate Does

Three kinds of work in a single pass:

1. **Path resolution** — scope-resolvable names only (locals, functions, enum
   cases, structs, type names). Type-dependent names (methods, fields on
   computed receivers) are left as strings for type inference.

2. **Desugaring** — syntactic sugar is eliminated here, before type inference
   ever sees it:
   - Binary/unary/compound-assign operators → `ProtocolCall` on protocol entities
   - `for x in collection { ... }` → `loop` + `iter()` + `next()` + `match`
   - `while cond { ... }` / `while let` → `loop` + `if !cond { break }`
   - `try expr` → `match` on `Ok`/`Err`
   - `throw value` → `return .Err(value)`
   - `value!` (unwrap) → `match` on `Some`/`None`
   - `"hello \(name)"` → `"hello " + name.description() + ""`
   - `guard let` → `if cond { } else { diverge }`
   - `if let pattern = expr` → `match expr { pattern => true, _ => false }`

3. **Local variable allocation** — params, `let`/`var` bindings, pattern
   bindings, and compiler-generated temporaries (`$iter`, `$let_tmp`,
   `$try_ok`, etc.) all get slots in the `HirBody.locals` arena.

## What This Crate Does NOT Do

- **Method resolution**: `x.foo()` becomes `HirExpr::MethodCall { receiver, method: "foo" }`.
  Which `foo` on which type? That's type inference's job.
- **Field resolution**: `x.bar` becomes `HirExpr::Field { base, name: "bar" }`.
  Same — the field entity is resolved later.
- **Overload resolution**: when `ResolveValuePath` returns multiple candidates,
  this crate picks the first and relies on type inference. There is no
  overload-set representation in HIR.
- **Type checking**: types are lowered (`AstType → HirTy`) but never checked
  against each other.

## Architecture

```
lib.rs          — LowerBody query entry point
ctx.rs          — LowerCtx: arenas, scope stack, local allocation
expr.rs         — Expression lowering, path resolution, call shape detection, Pratt parser
stmt.rs         — Statement lowering (let, expr, guard-let, deinit)
pat.rs          — Pattern lowering, literal parsing utilities
desugar.rs      — Operator/loop/try/throw/unwrap/interpolation desugaring
ty.rs           — AstType → HirTy, LowerTypeAnnotation/LowerCallableTypes queries
```

### Queries

| Query | Input | Output | Used by |
|---|---|---|---|
| `LowerBody` | `entity, root` | `Option<HirBody>` | `InferBody` (type inference) |
| `LowerTypeAnnotation` | `entity, root` | `Option<HirTy>` | `InferBody` (return type) |
| `LowerCallableTypes` | `entity, root` | `Option<Vec<Option<HirTy>>>` | `InferBody` (param types) |

`lower_ast_type` is also exported as a free function, used by type inference's
`WorldResolver` for where-clause type lowering.

### LowerCtx

All mutable state for one body lives in `LowerCtx`:

- **Arenas**: `Arena<HirExpr>`, `Arena<HirPat>`, `Arena<HirStmt>`, `Arena<Local>`
- **Scope stack**: `Vec<HashMap<String, LocalId>>` — lexical scoping via push/pop
- **Params**: `Vec<LocalId>` — parameter locals in declaration order
- **References**: `&QueryContext`, `root`, `owner` entity

## Design Decisions

### Operator precedence is applied here, not in the parser

The parser (chumsky-based) emits binary expressions as **flat, left-associative
chains** with no precedence applied. This is documented in the parser:
`Binary expression: a + b (flat, no precedence applied yet)`.

This crate corrects precedence via a Pratt parser in `expr.rs`:
1. `flatten_binary` recursively collects all operands and operators from the
   nested `AstExpr::Binary` tree into flat lists
2. `pratt_parse` re-assembles them with correct precedence and associativity
3. Each operator is then desugared to a `ProtocolCall` via `desugar_binary_hir`

This split is intentional — chumsky's combinator style makes left-recursive
precedence climbing awkward, and deferring it to the lowerer keeps the parser
simpler.

### Call shape detection: method calls vs direct calls

The parser can produce `local.method(args)` as either:
- `MemberAccess { base, member } + Call` — when the base is a complex expression
- `Path { segments: [local, method] } + Call` — when the base is a simple name

`lower_call` in `expr.rs` detects the second case by checking whether the
first path segment is a known local variable. If so, it rewrites to
`HirExpr::MethodCall`. Otherwise it falls through to a direct `HirExpr::Call`.

This heuristic is correct because:
- Locals shadow globals in Kestrel
- If the first segment isn't a local, it must be a type or module name, making
  this a static call (e.g., `MyType.staticMethod()`), which resolves through
  `ResolveValuePath` as a direct call

### Desugaring resolves protocol entities, not strings

Operator desugaring (e.g., `+` → `Addable.add`) resolves the protocol entity
at desugar time via `ResolveBuiltin`, producing `HirExpr::ProtocolCall` with a
concrete entity ID. This means:
- Type inference sees protocol calls, not raw operators
- If a protocol entity is missing (broken stdlib), the lowerer emits
  `HirExpr::Error` immediately rather than deferring the failure

### `Self` type resolution walks the owner hierarchy

`find_self_type` in `ty.rs` walks up from the current entity to find the
nearest `Struct`, `Enum`, or `Protocol`. For extensions, it resolves to the
extension's **target type** (via `ExtensionTargetEntity`), not the extension
entity itself. This means `Self` in an extension method refers to the type
being extended.

### Type alias transparency

Simple aliases like `type Fd = Int32` are resolved transparently during type
lowering: `lower_ast_type` checks for `NodeKind::TypeAlias` with a concrete
`TypeAnnotation` and recurses into the aliased type. This means `Fd` and
`Int32` produce the same `HirTy::Named` — they unify without any special logic
in type inference.

Abstract associated types (no `TypeAnnotation`) are left as
`HirTy::Named { entity: type_alias_entity }` for type inference to handle.

## Known Limitations

### Complex `let` destructuring is incomplete

For `let (a, b) = expr`, `stmt.rs` allocates a temp `$let_tmp`, lowers the
pattern (which defines `a` and `b` in scope via `define_local`), and creates a
match expression for destructuring — but the match statement is allocated and
then discarded (`let _match_stmt = ...`). Only the `let` binding for
`$let_tmp` is returned. The bindings exist in scope but are never assigned
from the temp.

### `@` binding patterns drop the outer binding

`pat.rs` handles `name @ subpattern` by calling `define_local` for `name`
(making it available in scope) but then returns only the lowered subpattern.
No HIR node captures the `name` binding, so it's defined but never written to.

### For-loop iterator is inside the loop body

`desugar.rs` places `let $iter = iterable.iter()` **inside** the `Loop` node
(as the first statement of the loop body). The comment says "so we return a
single expression." This means `$iter` is re-created on every iteration. The
correct desugaring would wrap the loop in a block: `{ let $iter = ...; loop { ... } }`.

### String interpolation uses `MethodCall`, not `ProtocolCall`

`desugar_interpolated_string` calls `.description()` via a plain
`HirExpr::MethodCall`, not a `ProtocolCall` through a `Describable`/`Formattable`
protocol. This means no conformance constraint is generated — the method is
resolved by type inference via ordinary member resolution.

### Overloaded functions are deferred to type inference

When `ResolveValuePath` returns `Overloaded(entities)`, `lower_path` emits
`HirExpr::OverloadSet { candidates, type_args, span }`. This preserves the
full overload set through HIR. Type inference detects `OverloadSet` as the
callee of a `Call` and emits a `Constraint::OverloadedCall` for the solver.

The solver resolves overloads in two steps:
1. **Label/arity filtering** — narrow candidates by matching arg labels and count
2. **Type compatibility** — if multiple candidates survive step 1 (e.g., inits
   that differ only by param type), wait for arg types to become concrete, then
   check structural type compatibility

Using an `OverloadSet` in non-call position (e.g., `let f = overloadedFunc`)
is an error — overloaded names can only be disambiguated at call sites.
