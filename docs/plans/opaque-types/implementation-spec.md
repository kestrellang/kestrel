# Opaque Types — Implementation Spec

This spec resolves every open question from the design critique. It describes
concrete data structures, control flow, and invariants — enough to implement
without further design work.

---

## Architecture Overview

Opaque types use a **two-view model**: inside the defining body, the return type
is a normal fresh TyVar that unifies with the concrete expression. Outside (at
call sites), the return type lowers to `TyKind::Opaque`, which delegates member
resolution to the protocol bounds but hides the concrete type.

Resolution happens **before MIR lowering**: `lower_resolved_ty` in
`kestrel-mir-lower/src/resolved_ty.rs` queries `InferBody` on the opaque origin
to retrieve the concrete type and substitutes it in. MIR and codegen never see
opaque types.

Category 4 (variable/property position: `let x: some P = ...`) is **deferred to
v2**. It inverts the normal annotation→initializer flow and requires
infrastructure that doesn't exist (concrete-type pinning from initializers,
var-reassignment type checking against a hidden concrete type). Categories 1–3
and return-position opaque types provide the core value.

### Key field/function names (verified against codebase)

- `InferCtx.owner` (NOT `body_owner`) — the entity being inferred (`ctx.rs:83`)
- `create_return_type` — queries `LowerTypeAnnotation`, returns TyVar (`lib.rs:328`)
- `lower_hir_ty_sub` — call-site return type lowering (`solver.rs:3527`)
- `build_result` — builds TypedBody from solved InferCtx (`result.rs:174`)
- `lower_resolved_ty` — ResolvedTy→MirTy conversion (`resolved_ty.rs:14`)
- `expand_protocol_closure_in_place` — free function, NOT a method (`name-res`)
- Query cycle detection: **panics** (`query.rs:360`), does NOT return None

---

## Data Structures

### Parser: `TyVariant::Some`

```rust
TyVariant::Some {
    some_span: Span,
    bounds: Vec<TyVariant>,        // P, Q from `some P and Q`
    negative: Option<TyVariant>,   // `and not Copyable`
}
```

Parsed as a **prefix** in `ty_parser()`, before the postfix operator chain.
`some` consumes the `Token::SomeKw` keyword, then parses a sequence of
path-with-args types separated by `Token::And`. If the last `and` is followed by
`Token::Not`, parse a negative bound.

Postfix operators chain normally: `some Shape?` parses as
`Optional(Some { bounds: [Shape] })`.

Emitted as `SyntaxKind::TySome` wrapping child type nodes.

### AST: `AstType::Some`

```rust
AstType::Some {
    bounds: Vec<AstType>,          // resolved path types for each bound
    negative_bound: Option<AstType>,
    span: Span,
}
```

Built from CST in `ast_type_from_cst` by matching `SyntaxKind::TySome`.

### HIR: `HirTy::Opaque`

```rust
HirTy::Opaque {
    bounds: Vec<HirTy>,           // HirTy::Protocol for each bound
    negative_bound: Option<HirTy>,
    span: Span,
}
```

Lowered from `AstType::Some` in `lower_ast_type`. Each bound is resolved via
`ResolveTypePath` and must resolve to a protocol entity. Non-protocol bounds
are diagnosed at this stage.

### Type Inference: `TyKind::Opaque`

```rust
TyKind::Opaque {
    origin: Entity,               // function/property that defines this
    bounds: Vec<(Entity, Vec<TyVar>)>,  // (protocol entity, protocol type args)
    origin_args: Vec<TyVar>,      // call-site type args for the origin function
    index: u32,                   // 0 for v1 (future: multi-some)
}
```

`bounds` is a `Vec`, not a single entity, to handle `some P and Q` natively
without synthetic intersection protocols. Each entry is a protocol entity with
its type arguments (e.g., `Iterator[Element = String]` stores the Iterator
entity + the constrained type args).

`origin_args` captures the call-site type arguments applied to the origin
function (e.g., for `wrap[Int64](42)` calling `wrap[T] -> some P`, origin_args
= `[Int64_TyVar]`). These are needed at MIR lowering time to substitute the
origin's type params in the concrete type.

### Inference Output: `ResolvedTy::Opaque`

```rust
ResolvedTy::Opaque {
    origin: Entity,
    bounds: Vec<(Entity, Vec<ResolvedTy>)>,
    origin_args: Vec<ResolvedTy>,  // resolved call-site type args
    index: u32,
}
```

Preserved through `kind_to_resolved` — NOT eagerly resolved to concrete at this
stage. `kind_to_resolved` takes only `&InferCtx<'_>` and has no `QueryContext`
to call `InferBody`, so it cannot resolve opaques. Instead it maps the
TyKind::Opaque fields to ResolvedTy equivalents and passes them through.

### MIR: No representation

`lower_resolved_ty` (in `kestrel-mir-lower/src/resolved_ty.rs`) resolves
`ResolvedTy::Opaque` to the concrete type. `LowerCtx` has `ctx.query`
(a `QueryContext`), so it can call `InferBody(origin)` to retrieve the concrete
return type, then substitute the origin's type params with `origin_args`.
MIR never sees opaque types.

---

## Phase 1: Parser

### Changes

1. **`lib/kestrel-lexer/src/lib.rs`**: Token is already `Token::Some`. Rename to
   `Token::SomeKw` to avoid Rust `Option::Some` confusion in error messages.
   Update all pattern references.

2. **`lib/kestrel-syntax-tree/src/lib.rs`**: Add `SyntaxKind::TySome`. Update
   `kind_from_raw()`.

3. **`lib/kestrel-parser/src/ty/mod.rs`**:
   - Add `TyVariant::Some { some_span, bounds, negative }`.
   - In `ty_parser()`, add a new alternative before `path`: when
     `Token::SomeKw` is seen, consume it, then parse bounds as
     `path_with_optional_args_parser()` separated by `Token::And`, with optional
     trailing `Token::Not` + path for negative bound.
   - Add `emit_some_type()` that wraps children in a `TySome` node.
   - Add `TySome` to `is_type_kind()` in `helpers.rs`.

4. **`lib/kestrel-ast-builder/src/ast_type.rs`**: Handle `SyntaxKind::TySome` in
   `ast_type_from_cst`. Extract child type nodes as bounds.

5. **`lib/kestrel-ast/src/ast_type.rs`**: Add `AstType::Some { bounds, negative_bound, span }`.

### Disambiguation: `some` in type vs pattern position

`some` is already used in pattern position (`some value` for Optional
unwrapping). No conflict: the parser knows whether it's parsing a type or a
pattern from context. Type parsing and pattern parsing are separate parser
entry points.

---

## Phase 2: HIR / Binder — Category 1 (Generic Sugar)

### Mechanism

In the AST builder, during `build_function` (or the equivalent function-entity
construction), scan parameter type annotations for `AstType::Some`. For each
occurrence:

1. **Spawn a synthetic TypeParam entity** using the existing pattern from
   `introduce_rhs_free_type_params` (`extension.rs:141-149`):
   ```rust
   let tp = world.spawn();
   world.set(tp, NodeKind::TypeParameter);
   world.set(tp, Name(format!("__opaque_{index}")));
   world.set(tp, FileId(file_entity));
   world.set(tp, DeclSpan(some_span));
   world.set(tp, CstNode(cst_ref));  // required — omitting breaks diagnostics
   world.set_parent(tp, function_entity);
   ```
   Note: `CstNode` is required. The real pattern in `introduce_rhs_free_type_params`
   sets it. For synthetic opaque params, use the CST node of the `some` keyword
   or the enclosing type annotation as the CstNode.

2. **Create WhereClause bounds** from each protocol in the `some` bounds:
   ```rust
   WhereConstraint::Bound {
       subject: AstType::Named(tp_path),
       protocols: bounds.clone(),
       node: ...,
   }
   ```

3. **Replace the parameter's AstType** from `Some { bounds }` to
   `Named { segment: __opaque_N }` referencing the synthetic TypeParam.

4. **Append to TypeParams**: merge into the function's existing `TypeParams`
   component.

Each `some` in the parameter list gets a **separate** synthetic type param.
Two `some Drawable` params produce two independent type params.

### Invisible to users

The synthetic type params cannot be named, turbofished, or referenced. They
use double-underscore names (`__opaque_0`, `__opaque_1`) which are not valid
user identifiers. Turbofish resolution only considers user-declared type params.

---

## Phase 3: HIR / Binder — Category 3 (Associated Type Sugar)

### Mechanism

During protocol analysis, scan each protocol method/property declaration's
return type for `AstType::Some`. For each occurrence:

1. **Create an anonymous associated type entity**:
   ```rust
   let assoc = world.spawn();
   world.set(assoc, NodeKind::TypeAlias);
   world.set(assoc, Name(format!("__{method_name}_Return")));
   world.set_parent(assoc, protocol_entity);
   // Add bound constraint
   world.set(assoc, WhereClause(vec![
       WhereConstraint::Bound { subject: assoc_type, protocols: bounds, .. }
   ]));
   ```

2. **Replace the return type** from `AstType::Some` to
   `AstType::Named` referencing the anonymous associated type.

3. **Register as associated type** in the protocol's child list.

Conforming types satisfy the associated type implicitly through their concrete
return type (Category 2 handling in the conforming type's body).

---

## Phase 4: Type Inference — Category 2 (Opaque Return)

This is the core mechanism. Two distinct views of the same return type.

### Internal view (inside the defining body)

In `create_return_type` (`lib/kestrel-type-infer/src/lib.rs`), when
`LowerTypeAnnotation` returns `HirTy::Opaque`:

1. **Create a fresh TyVar** (NOT `TyKind::Opaque`). This is the concrete
   return type that the body's expressions will unify with.
   ```rust
   let concrete_ret = ctx.fresh();
   ```

2. **Emit `Conforms` constraints** for each bound:
   ```rust
   for (protocol, args) in &opaque_bounds {
       ctx.emit(Constraint::Conforms {
           ty: concrete_ret,
           protocol: *protocol,
           span: opaque_span,
           poison_ty_on_failure: false,
       });
   }
   ```

3. **Store the opaque metadata** on InferCtx:
   ```rust
   ctx.opaque_return = Some(OpaqueReturnInfo {
       concrete_tv: concrete_ret,
       bounds: opaque_bounds,
       span: opaque_span,
   });
   ```

4. **Return `concrete_ret`** as the function's return TyVar. The body's tail
   expression and return statements unify with it normally.

### External view (at call sites)

In `lower_hir_ty_sub` (`lib/kestrel-type-infer/src/solver.rs`), when lowering
`HirTy::Opaque` for a callee's return type:

1. **Check self-reference**: if the opaque origin == `ctx.owner`, return
   `ctx.return_ty` directly. This gives recursive calls the internal concrete
   view, allowing the body's return type to unify with recursive call results.

2. **Otherwise, create `TyKind::Opaque`**:
   ```rust
   ctx.opaque(origin_entity, bounds_with_substituted_args, origin_args, 0)
   ```
   The bounds' type args are substituted through the callee's type parameter
   map (same as any other return type lowering). `origin_args` is the list of
   TyVars for the call-site type arguments (from the `subs` map passed to
   `lower_hir_ty_sub`). For non-generic functions, this is empty.

### Post-solve: extract concrete type

After `solver::solve()` completes, in `build_result`:

1. If `ctx.opaque_return` is `Some(info)`, resolve `info.concrete_tv` to get
   the concrete `ResolvedTy`.
2. Store it on `TypedBody`:
   ```rust
   pub struct TypedBody {
       // ... existing fields ...
       pub opaque_concrete_type: Option<ResolvedTy>,
   }
   ```
3. If the concrete type is still unresolved (e.g., all returns are recursive
   with no base case), emit E469 "cannot infer concrete type for opaque return."
4. **Update the `Hash` impl on `TypedBody`** (`result.rs:45-73`): TypedBody has
   a manual `Hash` that enumerates every field. The new
   `opaque_concrete_type` field must be added to the Hash impl, otherwise
   query memoization will be incorrect (the field is silently excluded from
   the hash, causing stale cache hits when the concrete type changes).

### Validation

After solving, validate that all return expressions yield the same concrete
type. Since they all unify with the same TyVar, this is enforced by
unification — conflicting types produce a unification error automatically.
The error message should be enhanced to say "all returns in a function with
opaque return type must have the same concrete type" (E468).

---

## Phase 5: Resolution (Opaque → Concrete)

### `kind_to_resolved` (preserves Opaque)

`kind_to_resolved` (`result.rs`) cannot resolve opaques — it takes only
`&InferCtx<'_>` with no `QueryContext`. Instead, it preserves
`ResolvedTy::Opaque` with resolved fields:

```rust
TyKind::Opaque { origin, bounds, origin_args, index } => {
    ResolvedTy::Opaque {
        origin: *origin,
        bounds: bounds.iter().map(|(e, args)| {
            (*e, args.iter().map(|&tv| resolve_to_concrete(ctx, tv)).collect())
        }).collect(),
        origin_args: origin_args.iter().map(|&tv| resolve_to_concrete(ctx, tv)).collect(),
        index: *index,
    }
}
```

### `lower_resolved_ty` (resolves Opaque → concrete)

Resolution happens in `lower_resolved_ty` (`resolved_ty.rs`), which has full
query access through `ctx.query`:

```rust
ResolvedTy::Opaque { origin, origin_args, .. } => {
    let body = ctx.query.query(InferBody { entity: *origin, root: ctx.root });
    let concrete = body
        .as_ref()
        .and_then(|b| b.opaque_concrete_type.as_ref())
        .cloned()
        .unwrap_or(ResolvedTy::Error);

    // Build substitution: origin's TypeParams → origin_args
    let type_params = ctx.world.get::<TypeParams>(*origin)
        .map(|tp| tp.0.clone())
        .unwrap_or_default();
    let substituted = substitute_resolved_ty(&concrete, &type_params, origin_args);
    lower_resolved_ty(ctx, &substituted)
}
```

Steps:
1. Query `InferBody(origin)` to get the `TypedBody`.
2. Extract `opaque_concrete_type` from the TypedBody.
3. Build substitution map from the origin's TypeParams to `origin_args`.
4. Apply substitution via `substitute_resolved_ty` helper (walks ResolvedTy,
   replacing `Param { entity }` with the matching `origin_args` element).
5. Recurse into `lower_resolved_ty` with the substituted concrete type.

### Cycle detection

The query system **panics** on re-entrant queries (`query.rs:360`). It does NOT
gracefully return `None`. This means mutual opaque recursion must be prevented
before it triggers a query cycle.

**Strategy**: two layers of defense.

1. **Self-recursion** is safe: the self-reference check in `lower_hir_ty_sub`
   (step 1 of external view) ensures recursive calls inside the body see
   `ctx.return_ty` directly, never triggering a re-entrant `InferBody` query.

2. **Mutual recursion** (`f -> g -> f`, both opaque returns): at MIR lowering
   time, `lower_resolved_ty` queries `InferBody(origin)`. If `f` and `g` are
   in different modules, their `InferBody` queries are independent — no cycle.
   If they're in the same module and mutual recursion exists, the cycle
   occurs during *inference* (not MIR lowering) when `lower_hir_ty_sub` in f's
   body creates an Opaque for g's return.

   Defense: wrap `InferBody.execute()` entry/exit with a thread-local
   `HashSet<Entity>` of in-progress entities. In `lower_hir_ty_sub`, before
   creating `TyKind::Opaque` for an external origin, check if `origin` is in
   the in-progress set. If so, it's mutual recursion — report E470 and return
   `ctx.error()` instead.

   This fires before the query system's panic-based cycle detection.

3. **No non-recursive return path**: handled by the post-solve check in Phase 4.
   If `ctx.opaque_return.concrete_tv` resolves to `TyKind::Error` or stays
   unresolved, emit E469.

---

## Solver Integration

### Unification rules

Add to `unify.rs`:

```
Opaque(A) == Opaque(B):
    if A.origin == B.origin && A.index == B.index && A.bounds.len() == B.bounds.len():
        pairwise unify A.bounds[i].args with B.bounds[i].args
    else:
        ERROR: type mismatch

Opaque(A) == Concrete:
    ERROR: type mismatch (opaque types hide their concrete identity)

Opaque(A) == Error:
    OK (poison absorption)
```

Two opaque types from different origins are **never** the same type, even if
they have the same bounds. `makeCircle() -> some Shape` and
`makeSquare() -> some Shape` produce distinct types.

### Member resolution

Add to `resolve.rs`, in the match on TyKind:

```rust
TyKind::Opaque { bounds, .. } => {
    // Expand bounds to include all superprotocols
    let mut all_protocols = Vec::new();
    for (protocol, _args) in bounds {
        all_protocols.push(*protocol);
    }
    // expand_protocol_closure_in_place is a FREE FUNCTION in kestrel-name-res,
    // NOT a method on the resolver. Call pattern:
    expand_protocol_closure_in_place(
        self.ctx, self.root, &mut all_protocols, &mut HashSet::new()
    );

    // Search direct members, then extension defaults, across all protocols
    for proto in &all_protocols {
        if let Some(member) = self.resolve_protocol_member(*proto, name) {
            return Some(member);
        }
    }
    for proto in &all_protocols {
        if let Some(member) = self.resolve_protocol_extension_member(*proto, name) {
            return Some(member);
        }
    }
    None
}
```

**Critical**: do NOT search extensions on the concrete type. Only the protocol
interface and protocol extensions are visible. This prevents leaking concrete
type identity through available methods.

### Conformance checking

Add to `conforms_to` in `resolve.rs`:

```rust
TyKind::Opaque { bounds, .. } => {
    // Expand all bounds through superprotocol chains
    let mut all_protocols = bounds.iter().map(|(p, _)| *p).collect::<Vec<_>>();
    expand_protocol_closure_in_place(
        self.ctx, self.root, &mut all_protocols, &mut HashSet::new()
    );
    all_protocols.contains(&target_protocol)
}
```

This allows opaque types to satisfy generic constraints:
```kestrel
func take[T: Shape](x: T) { ... }
let s = makeShape()    // some Shape
take(s)                // OK — opaque conforms to Shape
```

---

## Diagnostics

| ID   | Message                                                   | When                                               |
| ---- | --------------------------------------------------------- | -------------------------------------------------- |
| E466 | `some` is not allowed in this position                    | type alias, closure annotation, cast, where clause  |
| E467 | cannot access `{name}` on opaque type `some {P}`         | using concrete-type-only members                    |
| E468 | all returns must have the same concrete type              | conflicting return types (from unification error)    |
| E469 | cannot infer concrete type for opaque return              | no non-recursive return path                         |
| E470 | circular opaque type inference                            | mutual recursion cycle detected                      |
| E471 | stored property with `some` type requires an initializer  | deferred to v2 (Category 4)                          |
| E472 | concrete type `{X}` does not conform to `{P}`            | from Conforms constraint on return type              |

E472 may already be handled by the existing `DoesNotConform` inference error
(E465). If so, no new diagnostic needed — the existing conformance error fires
naturally because we emit `Conforms` constraints on the return TyVar.

### InferError wiring (5 files — per `kestrel-type-infer/AGENTS.md`)

Each new `InferError` variant must be mirrored in **five** files:

1. **`kestrel-type-infer/src/error.rs`** — add variant + span arm
2. **`kestrel-type-infer/src/result.rs`** — `describe_error()` arm
3. **`kestrel-compiler/src/diagnostic.rs`** — user-facing `Diagnostic`
4. **`kestrel-analyze/src/body/type_check.rs`** — `format_error()` arm
5. **`kestrel-compiler-driver/src/lib.rs`** — `describe()` + `format_error()`

Missing any of these produces a non-exhaustive match build error in a
downstream crate, discovered only after slow recompilation.

### Type display

In `describe_tykind` (`result.rs`), add:

```rust
TyKind::Opaque { bounds, .. } => {
    let bound_names: Vec<String> = bounds.iter()
        .map(|(entity, args)| format_protocol(ctx, *entity, args))
        .collect();
    format!("some {}", bound_names.join(" and "))
}
```

This displays as `some Shape`, `some Shape and Equatable`, etc. — clearly
distinguishable from bare protocol names.

---

## Pattern Matching

Pattern matching on opaque types is **prohibited**. The pattern matcher's
`TypeShape::classify` returns `TypeShape::Unknown` for opaque types (since
they have no enumerable constructors). This means:

- `match` on an opaque type produces an exhaustiveness error (no constructors
  to cover).
- Wildcard (`_`) patterns still work (they don't inspect the type).
- `is` checks against concrete types are out of scope (v1 doesn't support
  downcasting opaque types).

No explicit diagnostic needed — the existing exhaustiveness check handles it.
If we want a better message, add a special case in the exhaustiveness checker
that recognizes opaque types and emits "cannot pattern match on opaque type
`some P`" instead of the generic "non-exhaustive match."

---

## Witness Tables and Monomorphization

Opaque types are resolved to concrete types **during MIR lowering** (in
`lower_resolved_ty`). By the time monomorphization runs, all opaque types have
been replaced with their concrete types. No special handling is needed in the
monomorphizer or witness resolver.

The flow for `take(makeShape())`:
1. Inference: `take`'s T is unified with `some Shape` (opaque) via conformance
2. TypedBody records the call's type args as `[Opaque { origin: makeShape }]`
3. MIR lowering: `lower_resolved_ty` encounters the Opaque in the type args,
   queries `InferBody(makeShape)`, gets concrete type `Circle`, substitutes
4. MIR function def for the caller has `take[Circle]` as the callee
5. Monomorphization: instantiates `take[Circle]`
6. Witness resolution: finds `Circle`'s conformance to `Shape`, normal path

---

## Structural Positions

`some` inside type constructors works naturally:

```kestrel
func find() -> some Shape?           // Optional[Opaque{...}]
func items() -> Array[some Shape]    // Array[Opaque{...}]
```

The parser handles this because `some P` is parsed as a base type, and `?` is
a postfix operator. `Array[some Shape]` is a generic type arg.

At each level, the opaque type is represented as `TyKind::Opaque` nested inside
`TyKind::Struct { entity: Optional/Array, args: [Opaque{...}] }`.

During MIR resolution, `lower_resolved_ty` recursively processes type arguments,
so opaque types nested inside containers are resolved automatically.

### One-`some`-per-function rule (v1)

Add a check during HIR lowering or binder: count occurrences of `HirTy::Opaque`
in the return type tree. If > 1, emit E466. This applies only to return position
(Category 2); Category 1 (parameters) allows multiple `some` naturally since
each desugars to an independent type param.

### Illegal positions

`some` in the **parameter** position of a returned function type is illegal:
```kestrel
func f() -> (some Shape) -> Void    // ERROR: E466
```

Check: when lowering a `HirTy::Function` in return position, if any parameter
type contains `HirTy::Opaque`, emit E466. The caller would need to produce a
value of the hidden type, which is impossible.

---

## Incremental Compilation

Opaque types don't introduce new invalidation concerns. The concrete type is
stored inside `TypedBody` (output of `InferBody` query), which is already
memoized and invalidated when the function body changes. Call sites that use
the opaque return type re-query `InferBody` during their own MIR lowering,
which triggers recomputation if the origin changed.

This is identical to how changing a function's return type already invalidates
callers through the query dependency graph.

---

## `any P` Interaction

`some P` and `any P` (existential types) are distinct type-system concepts:

- `some P`: caller-opaque, compiler-known concrete type, static dispatch
- `any P`: runtime-boxed existential, vtable dispatch

`any P` is not implemented in Kestrel. When it is, `some P` values should NOT
implicitly convert to `any P` (this would require boxing). Explicit conversion
may be added in the future.

The `TyKind::Opaque` variant is specifically for `some` — `any` would use a
separate `TyKind::Existential` or similar.

---

## Associated Type Visibility Through Opaque Types

### Unconstrained associated types

If a protocol has associated types and the opaque bound doesn't constrain them,
they remain opaque but consistent:

```kestrel
func items() -> some Iterable {
    [1, 2, 3]
}
let it = items()
let a = it.next()    // type: AssocProjection(Opaque{Iterable}, "Element")
let b = it.next()    // same type
a == b               // ERROR — Element not known to be Equatable
```

Implementation: when member resolution on `Opaque{Iterable}` returns a method
whose return type references an associated type, that associated type is
projected through the opaque type. The projection base is the opaque type
itself, creating `AssocProjection { base: Opaque{...}, assoc: Element }`.

### Constrained associated types

```kestrel
func items() -> some Iterable[Element = Int64] { ... }
```

The associated type constraint is stored in the opaque bound's type args. When
resolving `AssocProjection { base: Opaque{Iterable[Element=Int64]}, assoc: Element }`,
the solver checks if Element is constrained in the bounds and resolves it to
Int64 directly.

---

## Error Propagation

If opaque type inference fails (e.g., body doesn't conform to bounds), the
`Conforms` constraint on the return TyVar fires and produces an `InferError`.
This does NOT poison the TyVar — it stays as whatever concrete type the body
inferred. The error is reported, but callers that reference the opaque return
type still see `TyKind::Opaque` and can resolve members through the protocol
bounds.

This prevents cascade errors: the caller's code isn't wrong (it correctly uses
protocol methods), only the implementation is wrong (it doesn't conform).

---

## Summary of Files to Modify

| File | Change |
|------|--------|
| `kestrel-lexer/src/lib.rs` | Rename Token::Some → Token::SomeKw |
| `kestrel-syntax-tree/src/lib.rs` | Add SyntaxKind::TySome, update kind_from_raw |
| `kestrel-parser/src/ty/mod.rs` | Add TyVariant::Some, parse logic, emit_some_type |
| `kestrel-ast-builder/src/builders/helpers.rs` | Add TySome to is_type_kind |
| `kestrel-ast-builder/src/ast_type.rs` | Handle TySome in ast_type_from_cst |
| `kestrel-ast/src/ast_type.rs` | Add AstType::Some variant |
| `kestrel-hir-lower/src/ty.rs` | Add HirTy::Opaque lowering from AstType::Some |
| `kestrel-hir/src/ty.rs` | Add HirTy::Opaque variant |
| `kestrel-ast-builder/src/builders/function.rs` | Category 1: desugar_opaque_params |
| `kestrel-ast-builder/src/builders/protocol.rs` | Category 3: desugar opaque associated types |
| `kestrel-type-infer/src/ty.rs` | Add TyKind::Opaque variant |
| `kestrel-type-infer/src/lib.rs` | create_return_type: handle HirTy::Opaque |
| `kestrel-type-infer/src/ctx.rs` | Add opaque_return: Option<OpaqueReturnInfo> |
| `kestrel-type-infer/src/solver.rs` | lower_hir_ty_sub: handle HirTy::Opaque |
| `kestrel-type-infer/src/unify.rs` | Add Opaque unification rules |
| `kestrel-type-infer/src/resolve.rs` | Add Opaque member resolution + conformance |
| `kestrel-type-infer/src/result.rs` | Add ResolvedTy::Opaque, kind_to_resolved, describe |
| `kestrel-mir-lower/src/resolved_ty.rs` | Resolve Opaque → concrete via InferBody query + substitution |
| `kestrel-pattern-matching/src/constructor.rs` | Existing `_ => Unknown` handles Opaque automatically |
| `kestrel-type-infer/src/error.rs` | New InferError variants + span arms |
| `kestrel-type-infer/src/result.rs` | describe_error arms for new variants |
| `kestrel-compiler/src/diagnostic.rs` | User-facing Diagnostic construction |
| `kestrel-analyze/src/body/type_check.rs` | format_error arms for new variants |
| `kestrel-compiler-driver/src/lib.rs` | describe + format_error arms |

---

## Implementation Order

1. **Parser** (Phase 1) — can be tested with parse-tree snapshot tests
2. **Category 1 desugaring** (Phase 2) — test with existing generic infrastructure
3. **TyKind::Opaque + internal/external views** (Phase 4) — core mechanism
4. **Member resolution + conformance** (Phase 4 cont.) — enables calling methods
5. **MIR resolution** (Phase 5) — enables codegen
6. **Category 3 desugaring** (Phase 3) — protocol requirement sugar
7. **Diagnostics** (Phase 7) — error messages for all failure modes
8. **Structural positions + restrictions** (Phase 4 cont.) — one-some rule, illegal positions

Each phase can be tested independently before moving to the next.
