# HIR Lowering: `kestrel-hir-lower`

## Pipeline Position

```
CST ──→ Declaration builder (mutation) → ECS World
 │                                          ↓
 │                                    Name Resolution (queries)
 │                                          ↓
 └── body syntax (`Valued` ptr) ──→ HIR Lowering (this crate) ← LowerBody query
                                            ↓
                                    Type Inference (InferBody query)
```

HIR lowering converts a body's **syntax** — read through the typed views, with
no intermediate AST — into `HirBody` (partially-resolved HIR), plus a
`BodySourceMap`. It sits between name resolution and type inference, consuming
both:

- **From the declaration builder**: `Valued` (a pointer to the body syntax,
  resolved against the file's `FileSyntax`) and `Callable` components on entities
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
   - `while cond { ... }` → `loop` + `if cond {} else { break }`; `while let` →
     `loop { match v { p => body, _ => break } }`
   - `try expr` → `match expr.tryExtract() { .Continue(v) => v, .Break(e) => return .fromResidual(e) }`
   - `throw value` → `return .Err(value)`
   - `value!` (unwrap) → `ForceUnwrap.forceUnwrap()` protocol call
   - `"hello \(name)"` → a `DefaultStringInterpolation` builder (`appendLiteral` /
     `appendInterpolation` / `build`)
   - `guard let` → CPS: the rest of the block is the match's success arm
   - `if let pattern = expr` → nested `match`es threading bindings into the then-block

3. **Local variable allocation** — params, `let`/`var` bindings, pattern
   bindings, and compiler-generated temporaries (`$iter`, `$let_tmp`,
   `$try_value`, etc.) all get slots in the `HirBody.locals` arena.

## What This Crate Does NOT Do

- **Method resolution**: `x.foo()` becomes `HirExpr::MethodCall { receiver, method: "foo" }`.
  Which `foo` on which type? That's type inference's job.
- **Field resolution**: `x.bar` becomes `HirExpr::Field { base, name: "bar" }`.
  Same — the field entity is resolved later.
- **Overload resolution**: when `ResolveValuePath` returns multiple candidates,
  this crate emits `HirExpr::OverloadSet` and type inference picks.
- **Type checking**: types are lowered (`AstType → HirTy`) but never checked
  against each other.

## Architecture

```
lib.rs          — LowerBody / LowerBodyWithSourceMap, body-node dispatch, parameters
syntax.rs       — reading the CST (blocks, conditions, paths, operators, implicit `it`)
source_map.rs   — BodySourceMap
ctx.rs          — LowerCtx: arenas, scope stack, local allocation
expr.rs         — Expression lowering, path resolution, call shape detection
stmt.rs         — Statement lowering (let, expr, guard, deinit)
pat.rs          — Pattern lowering, literal parsing utilities
desugar.rs      — Operator/loop/try/throw/interpolation desugaring
ty.rs           — AstType → HirTy, LowerTypeAnnotation/LowerCallableTypes queries
```

### Queries

| Query | Input | Output | Used by |
|---|---|---|---|
| `LowerBodyWithSourceMap` | `entity, root` | `Option<Arc<LoweredBody>>` | `LowerBody`; the LSP (cursor ↔ HIR) |
| `LowerBody` | `entity, root` | `Option<Arc<HirBody>>` | `InferBody` (type inference), analyzers, MIR |
| `LowerTypeAnnotation` | `entity, root` | `Option<HirTy>` | `InferBody` (return type) |
| `LowerCallableTypes` | `entity, root` | `Option<Vec<Option<HirTy>>>` | `InferBody` (param types) |

`lower_ast_type` is also exported as a free function, used by type inference's
`WorldResolver` for where-clause type lowering.

### LowerCtx

All mutable state for one body lives in `LowerCtx`:

- **Arenas**: `Arena<HirExpr>`, `Arena<HirPat>`, `Arena<HirStmt>`, `Arena<Local>`
- **Scope stack**: `Vec<HashMap<String, LocalId>>` — lexical scoping via push/pop
- **Params**: `Vec<LocalId>` — parameter locals in declaration order
- **References**: `&QueryContext`, `root`, `owner` entity, `file_id`
- **Source map**: the `BodySourceMap` being recorded

## Design Decisions

### Operator precedence is the parser's

The parser applies precedence and associativity (`kestrel-parser`
`grammar/exprs.rs::binary_binding_power` is the operator table), so the CST,
and the spans already have the final shape. Lowering takes each `ExprBinary` as
written and desugars it to a `ProtocolCall` via
`desugar_binary_hir`, with the node's own span. (It used to flatten a
left-folded chain and re-associate it here, which gave mixed-precedence
operators the wrong spans — audit H6.)

### Call shape detection: method calls vs direct calls

The parser produces `local.method(args)` as an `ExprCall` whose callee is an
`ExprPath`, either:
- a member access on a computed base (`f().method(args)`), when the base is an
  expression — `lower_member_call`;
- a pure path (`local.method`), when every segment is an identifier —
  `lower_path_call`.

For the second, `lower_path_call` checks whether the first path segment is a
known local variable. If so, it rewrites to `HirExpr::MethodCall`. Otherwise
it tries a static method on a type, a type-level call, a value prefix, and
falls through to a direct `HirExpr::Call`.

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

### Lowering reads the CST directly

There is no body AST. `LowerBodyWithSourceMap` resolves the entity's `Valued`
pointer and walks the typed views; the decisions that take more than one
accessor live in `syntax.rs` so the lowering code reads as "what does this mean":

- **Blocks**: a `Statement` child is a statement; a trailing bare `Expression`,
  or a final statement-like `ExpressionStatement` without `;`, is the value
  (`block_syntax`). A closure's items sit directly in `ExprClosure`, where a
  statement-like expression stands *without* a `Statement` wrapper; one that is
  not last is demoted to a statement in source order (`closure_body_syntax`).
- **Paths**: an `ExprPath` is either identifier segments (`PathBase::Segments`)
  or a computed base plus member accesses (`PathBase::Expr`). Whether a segment
  is a value or a member is decided by *scope* in `lower_path`, never by syntax.
- **Implicit `it`**: a closure without a parameter header gets an `it` parameter
  when its body refers to the **name** `it` (a value path whose first segment is
  `it`), looking through nested closures that declare parameters and stopping
  at nested header-less ones (`implicit_it_reference`). The implicit parameter
  always wins over an outer `it`; E142 (an enclosing closure's `it`) / E143
  (another outer binding) warn at the first reference.
- **Missing syntax** lowers to explicit error/missing forms, never an empty
  name: `ExprSrc::Error(span)` / `PatSrc::Error(span)` → `HirExpr::Error` /
  `HirPat::Error`; an absent member or case name → `HirName::Missing`; a binder
  without a name → `HirPat::Error`; a nameless `deinit` reports nothing more.
- **Allocation order** of HIR ids follows source order exactly as before the
  port (callers rely on it: diagnostics, `while_conditions`, analyzers).

### The source map

`BodySourceMap` is recorded by the same walk, so ids and syntax can never
disagree:

- every node lowered through `lower_expr` / `lower_pat` / `lower_stmt` ↔ its id
  (a desugaring's node maps to its outermost id; a grouping `(e)` maps to `e`'s);
- every local the source spells → its binding node + identifier
  (`LocalSource`), and an identifier position → that local. Function and
  initializer parameters are matched to their signature's `BindingPattern`s.
  Not recorded: `self`, an implicit `it`, desugaring temporaries, destructured
  parameters' synthetic `_param_N`, struct-pattern shorthand `{ x }` (the token
  is also the field name) and subscript index parameters (bound again by every
  accessor body) — tools must fail closed on those;
- every path segment / member name that lowers to an expression of its own
  (`Local`, a type parameter's `Def`, a `Field`, a callee prefix's receiver or
  static method) → that expression (`name_refs`), since segments are tokens,
  not nodes.

`Local::span` keeps its meaning (the whole declaration — type inference anchors
"could not infer type" there, see `docs/fragility/F2/decisions.md`); the name
lives in the source map.

## Known Limitations

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
