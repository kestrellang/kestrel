# Type Inference: `kestrel-type-infer`

## Pipeline Position

```
CST → AST → build_declarations (mutation)
                ↓
          World with entities + components
                ↓
          Name Resolution queries
                ↓
          HIR Lowering (LowerBody query → HirBody)
                ↓
          Type Inference (this crate)  ← NEW
                ↓
          TypedBody (types + resolutions)
                ↓
          Validation / Codegen (future)
```

## Architecture Overview

```
┌─────────────────────────────────────────┐
│  InferBody query                        │
│  (entry point per function/init/getter) │
└────────────────┬────────────────────────┘
                 │
    ┌────────────┼────────────┐
    │            │            │
    v            v            v
 generate.rs  solver.rs   resolve.rs
 (walk HIR,   (fixpoint   (member/conformance
  emit        iteration)   resolution via
  constraints)             queries)
    │            │            │
    └────────────┼────────────┘
                 │
                 v
         TypedBody output
         (type tables + resolution tables)
```

## Crate Structure

```
lib2/kestrel-type-infer/
  Cargo.toml
  src/
    lib.rs        — InferBody query, crate docs
    ty.rs         — TyVar, TyKind, type representation
    constraint.rs — Constraint enum
    ctx.rs        — InferCtx (solver state, TyVar allocation)
    generate.rs   — Constraint generation (HirBody → constraints)
    solver.rs     — Fixpoint solver loop
    unify.rs      — Unification with literal guards, Error/Never
    resolve.rs    — TypeResolver trait + impl over QueryContext
    result.rs     — TypedBody output, InferResult
    error.rs      — InferError, diagnostics
```

---

## Core Types

### Type Variables (`ty.rs`)

```rust
/// Index into InferCtx::types. Cheap to copy.
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
struct TyVar(u32);

/// What we know about a type variable.
enum TySlot {
    /// Fresh, unconstrained. May have a literal marker.
    Unresolved { literal: Option<LiteralKind> },

    /// Redirects to another TyVar (unification link).
    Redirect(TyVar),

    /// Bound to a concrete type.
    Resolved(TyKind),
}

/// Concrete type representation used by the solver.
enum TyKind {
    /// Named type (struct, enum, protocol): entity + type args.
    Named { entity: Entity, args: Vec<TyVar> },

    /// Type parameter from a generic declaration.
    Param { entity: Entity },

    /// Tuple type.
    Tuple(Vec<TyVar>),

    /// Function type: (params) → return.
    Function { params: Vec<TyVar>, ret: TyVar },

    /// The Never type (bottom): unifies with anything,
    /// represents diverging control flow.
    Never,

    /// Error poison: unifies with anything silently.
    /// Only created after an error is reported.
    Error,
}

/// Literal kind marker on unresolved TyVars.
/// Controls which ExpressibleBy* protocol is required
/// and which default type alias to apply.
enum LiteralKind {
    Integer,
    Float,
    String,
    Bool,
    Char,
    Null,
    Array,       // default: @builtin(.DefaultArrayLiteralType)[_]
    Dictionary,  // default: @builtin(.DefaultDictionaryLiteralType)[_, _]
}
```

### How TyVars Work

Every type in the system is a `TyVar` — an index into a flat `Vec<TySlot>`. This eliminates the old system's HashMap-based type registry and substitution chains.

**Creating types:**
```rust
// Fresh inference variable
let tv = ctx.fresh();                    // TySlot::Unresolved { literal: None }

// Literal inference variable
let tv = ctx.fresh_literal(Integer);     // TySlot::Unresolved { literal: Some(Integer) }

// Concrete type
let tv = ctx.named(int64_entity, vec![]); // TySlot::Resolved(Named { entity, args })

// Error (must report error first)
let tv = ctx.report_error(err);          // emits diagnostic, returns Error TyVar
```

**Resolving types (find the root):**
```rust
fn resolve(&self, tv: TyVar) -> TyVar {
    match &self.types[tv] {
        TySlot::Redirect(target) => self.resolve(*target), // follow chain
        _ => tv,                                            // found root
    }
}
```

Path compression can be added as an optimization but isn't required for correctness.

---

## Constraints (`constraint.rs`)

Six constraint variants cover the entire type system:

```rust
enum Constraint {
    /// τ₁ = τ₂ — structural type equality.
    ///
    /// Used where types must be identical:
    /// if/match branches, array elements, tuple construction.
    Equal {
        a: TyVar,
        b: TyVar,
        span: Span,
    },

    /// τ₁ → τ₂ — value flows from source to target.
    ///
    /// Tries Equal first. If that fails because of a literal guard
    /// or structural mismatch, falls back to promotion (FromValue).
    ///
    /// Used at value boundaries: let bindings, function arguments,
    /// return statements, assignments.
    Coerce {
        from: TyVar,
        to: TyVar,
        expr: HirExprId,
        span: Span,
    },

    /// τ : Protocol — protocol conformance.
    ///
    /// Deferred until τ is concrete. Used for:
    /// - Where clause bounds (where T: Equatable)
    /// - Operator desugaring (receiver must conform to protocol)
    /// - Literal protocol checks
    Conforms {
        ty: TyVar,
        protocol: Entity,
        span: Span,
    },

    /// Container.Name → τ — associated type projection.
    ///
    /// Deferred until Container is concrete, then resolved
    /// via the type resolver. Used for:
    /// - Iterator.Item, Collection.Element, etc.
    /// - Where clause equalities (where T.Item = Int)
    Associated {
        container: TyVar,
        name: String,
        result: TyVar,
        span: Span,
    },

    /// receiver.name(args) → τ — member resolution.
    ///
    /// Covers methods, fields, subscripts, and inits.
    /// Deferred until receiver type is concrete, then resolved
    /// via the type resolver. Emits where clause constraints
    /// from the resolved member.
    Member {
        receiver: TyVar,
        name: String,
        args: Vec<CallArg>,
        result: TyVar,
        expr: HirExprId,
        span: Span,
    },

    /// .name(args) → τ — implicit enum member.
    ///
    /// Resolved against the expected type (e.g., .Some(x) where
    /// context expects Optional[T]). Deferred until expected
    /// type is concrete.
    Implicit {
        expected: TyVar,
        name: String,
        args: Vec<CallArg>,
        result: TyVar,
        expr: HirExprId,
        span: Span,
    },
}

/// Argument in a call: optional label + type.
struct CallArg {
    label: Option<String>,
    ty: TyVar,
}
```

---

## Solver State (`ctx.rs`)

```rust
struct InferCtx<'a> {
    /// Type resolver for querying the world.
    resolver: &'a dyn TypeResolver,

    /// All type variables. Index = TyVar(n).
    types: Vec<TySlot>,

    /// Pending constraints.
    constraints: Vec<Constraint>,

    /// Accumulated errors (each produces an Error TyVar).
    errors: Vec<InferError>,

    // === Results (populated during solving) ===

    /// Resolved entity for MethodCall/Field expressions.
    resolutions: HashMap<HirExprId, Entity>,

    /// Promotion info for Coerce sites that needed wrapping.
    promotions: HashMap<HirExprId, PromotionInfo>,

    /// Inferred type arguments for generic calls.
    type_args: HashMap<HirExprId, Vec<TyVar>>,

    // === Bookkeeping ===

    /// Type assigned to each HirExpr during constraint gen.
    expr_types: HashMap<HirExprId, TyVar>,

    /// Type assigned to each Local during constraint gen.
    local_types: HashMap<LocalId, TyVar>,

    /// The function's declared return type TyVar.
    return_ty: TyVar,

    /// Entity being inferred (for context in resolver calls).
    owner: Entity,
    root: Entity,
}
```

### Key Methods

```rust
impl InferCtx<'_> {
    // === TyVar creation ===
    fn fresh(&mut self) -> TyVar;
    fn fresh_literal(&mut self, kind: LiteralKind) -> TyVar;
    fn named(&mut self, entity: Entity, args: Vec<TyVar>) -> TyVar;
    fn tuple(&mut self, elements: Vec<TyVar>) -> TyVar;
    fn function(&mut self, params: Vec<TyVar>, ret: TyVar) -> TyVar;
    fn never(&mut self) -> TyVar;

    // === Error reporting (returns Error TyVar) ===
    fn report_error(&mut self, err: InferError) -> TyVar;

    // === Resolution ===
    fn resolve(&self, tv: TyVar) -> TyVar;          // follow redirect chain
    fn slot(&self, tv: TyVar) -> &TySlot;           // resolve + return slot
    fn is_concrete(&self, tv: TyVar) -> bool;        // resolved to non-Infer?
    fn is_error(&self, tv: TyVar) -> bool;           // resolved to Error?

    // === Constraint emission ===
    fn equal(&mut self, a: TyVar, b: TyVar, span: Span);
    fn coerce(&mut self, from: TyVar, to: TyVar, expr: HirExprId, span: Span);
    fn conforms(&mut self, ty: TyVar, protocol: Entity, span: Span);
    fn associated(&mut self, container: TyVar, name: &str, result: TyVar, span: Span);
    fn member(&mut self, receiver: TyVar, name: &str, args: Vec<CallArg>,
              result: TyVar, expr: HirExprId, span: Span);
    fn implicit(&mut self, expected: TyVar, name: &str, args: Vec<CallArg>,
                result: TyVar, expr: HirExprId, span: Span);
}
```

---

## Type Resolver (`resolve.rs`)

Slim trait (5 methods) that abstracts world queries for testability:

```rust
trait TypeResolver {
    /// Look up a member on a type by name, with argument info for
    /// overload resolution. Returns the member entity, its type,
    /// and any type parameter instantiation info.
    ///
    /// Handles: methods, fields, computed properties, subscripts,
    /// inits, extension methods. Searches direct children and
    /// applicable extensions.
    fn resolve_member(
        &self,
        receiver_ty: &TyKind,
        name: &str,
        args: &[CallArg],
    ) -> Result<MemberResolution, MemberError>;

    /// Check if a concrete type conforms to a protocol.
    /// Walks conformance declarations and extension conformances.
    fn conforms_to(&self, ty: &TyKind, protocol: Entity) -> bool;

    /// Resolve an associated type on a container.
    /// e.g., Array[Int].Element → Int
    fn resolve_associated_type(
        &self,
        container: &TyKind,
        name: &str,
    ) -> Option<AssociatedTypeResolution>;

    /// Look up a builtin entity by language feature.
    /// Used for literal protocols, default types, operators.
    fn builtin(&self, feature: BuiltinFeature) -> Option<Entity>;

    /// Get the where clauses for an entity (function, method, init).
    /// Returns bounds and type equalities.
    fn where_clauses(&self, entity: Entity) -> Vec<WhereClause>;
}

struct MemberResolution {
    /// The resolved entity (function, field, getter, etc.)
    entity: Entity,
    /// Type parameters of the member (to be instantiated with fresh TyVars)
    type_params: Vec<Entity>,
    /// Parameter types (with type param placeholders)
    param_types: Vec<TyKind>,
    /// Return type (with type param placeholders)
    return_type: TyKind,
    /// Where clauses on this member
    where_clauses: Vec<WhereClause>,
    /// Whether this is a field, method, computed property, etc.
    kind: MemberKind,
}

enum MemberKind {
    Field { mutable: bool },
    Method,
    ComputedProperty { has_setter: bool },
    Subscript,
    Init,
}

enum MemberError {
    NotFound,
    Ambiguous(Vec<Entity>),
    NotVisible,
}

struct AssociatedTypeResolution {
    /// The concrete type this associated type resolves to
    resolved: TyKind,
    /// Type param entities used (for creating TyVars)
    type_params: Vec<Entity>,
}

enum WhereClause {
    /// T: Protocol
    Bound { param: Entity, protocol: Entity },
    /// T.Item = SomeType
    TypeEquality { left: AssociatedTypePath, right: TypeRef },
}
```

### Implementation Over QueryContext

```rust
struct WorldResolver<'a> {
    ctx: &'a QueryContext<'a>,
    root: Entity,
    owner: Entity,
}

impl TypeResolver for WorldResolver<'_> {
    fn resolve_member(&self, receiver_ty: &TyKind, name: &str, args: &[CallArg])
        -> Result<MemberResolution, MemberError>
    {
        let TyKind::Named { entity, args: type_args } = receiver_ty else {
            return Err(MemberError::NotFound);
        };

        // 1. Search direct children by name
        let children = self.ctx.query(VisibleChildrenByName {
            parent: *entity, name: name.into(), context: self.owner,
        });

        // 2. Search extensions
        let extensions = self.ctx.query(ExtensionsFor {
            target: *entity, root: self.root,
        });
        // ... filter extension children by name, check applicability ...

        // 3. Overload resolution by args/labels
        // ... score candidates, pick best match ...

        // 4. Build MemberResolution
        Ok(MemberResolution { entity, type_params, param_types, return_type, ... })
    }

    // ... other methods use existing name-res queries ...
}
```

---

## Constraint Generation (`generate.rs`)

Walk the `HirBody` once, emitting constraints for every node.

### Entry Point

```rust
fn generate(ctx: &mut InferCtx, hir: &HirBody) {
    // Create TyVars for params (type known from Callable component)
    for &param_local in &hir.params {
        let param_ty = type_from_callable(ctx, param_local);
        ctx.local_types.insert(param_local, param_ty);
    }

    // Set return type from declaration
    ctx.return_ty = return_type_from_callable(ctx);

    // Generate constraints for statements
    for &stmt_id in &hir.statements {
        gen_stmt(ctx, hir, stmt_id);
    }

    // Generate constraints for tail expression
    if let Some(tail) = hir.tail_expr {
        let tail_tv = gen_expr(ctx, hir, tail);
        // Tail expression flows to return type
        ctx.coerce(tail_tv, ctx.return_ty, tail, hir.exprs[tail].span());
    }
}
```

### Expression Generation

```rust
fn gen_expr(ctx: &mut InferCtx, hir: &HirBody, id: HirExprId) -> TyVar {
    let tv = match &hir.exprs[id] {

        // === Literals ===
        HirExpr::Literal { value, .. } => match value {
            HirLiteral::Integer(_) => ctx.fresh_literal(LiteralKind::Integer),
            HirLiteral::Float(_)   => ctx.fresh_literal(LiteralKind::Float),
            HirLiteral::String(_)  => ctx.fresh_literal(LiteralKind::String),
            HirLiteral::Bool(_)    => ctx.fresh_literal(LiteralKind::Bool),
            HirLiteral::Char(_)    => ctx.fresh_literal(LiteralKind::Char),
            HirLiteral::Null       => ctx.fresh_literal(LiteralKind::Null),
        },

        // === References ===
        HirExpr::Local(local_id, _) => {
            // Look up the TyVar assigned when this local was declared
            ctx.local_types[local_id]
        },

        HirExpr::Def(entity, _) => {
            // Read the entity's type from the world.
            // For generic entities, instantiate fresh TyVars.
            instantiate_entity(ctx, *entity)
        },

        // === Calls ===
        HirExpr::Call { callee, args, span, .. } => {
            let callee_tv = gen_expr(ctx, hir, *callee);
            let arg_tvs: Vec<CallArg> = gen_call_args(ctx, hir, args);
            let result_tv = ctx.fresh();

            // Build expected function type: (arg_tys) → result_tv
            let param_tvs: Vec<TyVar> = arg_tvs.iter().map(|a| a.ty).collect();
            let fn_tv = ctx.function(param_tvs, result_tv);

            // Callee type must match the function type
            ctx.equal(callee_tv, fn_tv, span.clone());
            result_tv
        },

        HirExpr::MethodCall { receiver, method, args, span, .. } => {
            let recv_tv = gen_expr(ctx, hir, *receiver);
            let arg_tvs = gen_call_args(ctx, hir, args);
            let result_tv = ctx.fresh();

            // Member constraint: receiver.method(args) → result
            ctx.member(recv_tv, method, arg_tvs, result_tv, id, span.clone());
            result_tv
        },

        HirExpr::ProtocolCall { receiver, protocol, method, args, span, .. } => {
            let recv_tv = gen_expr(ctx, hir, *receiver);
            let arg_tvs = gen_call_args(ctx, hir, args);
            let result_tv = ctx.fresh();

            // Receiver must conform to the protocol
            ctx.conforms(recv_tv, *protocol, span.clone());
            // Resolve method on the protocol
            ctx.member(recv_tv, method, arg_tvs, result_tv, id, span.clone());
            result_tv
        },

        // === Member Access ===
        HirExpr::Field { base, name, span, .. } => {
            let base_tv = gen_expr(ctx, hir, *base);
            let result_tv = ctx.fresh();

            // Member constraint with no args (field/property access)
            ctx.member(base_tv, name, vec![], result_tv, id, span.clone());
            result_tv
        },

        HirExpr::TupleIndex { base, index, span, .. } => {
            let base_tv = gen_expr(ctx, hir, *base);
            let result_tv = ctx.fresh();
            // Special: extract element from tuple type during solving
            // Emit as Equal with a tuple projection
            ctx.constraints.push(Constraint::TupleIndex {
                tuple: base_tv, index: *index, result: result_tv, span: span.clone(),
            });
            result_tv
        },

        // === Implicit Member (.CaseName) ===
        HirExpr::ImplicitMember { name, args, span, .. } => {
            let arg_tvs = args.as_ref()
                .map(|a| gen_call_args(ctx, hir, a))
                .unwrap_or_default();
            let result_tv = ctx.fresh();

            // Implicit constraint: resolved against expected type
            ctx.implicit(result_tv, name, arg_tvs, result_tv, id, span.clone());
            result_tv
        },

        // === Control Flow ===
        HirExpr::If { condition, then_block, else_expr, span, .. } => {
            let cond_tv = gen_expr(ctx, hir, *condition);
            let bool_tv = ctx.named(ctx.resolver.builtin(Bool).unwrap(), vec![]);
            ctx.equal(cond_tv, bool_tv, span.clone());

            let then_tv = gen_block(ctx, hir, then_block);

            if let Some(else_id) = else_expr {
                let else_tv = gen_expr(ctx, hir, *else_id);
                let result_tv = ctx.fresh();
                // Both branches must agree
                ctx.equal(then_tv, result_tv, span.clone());
                ctx.equal(else_tv, result_tv, span.clone());
                result_tv
            } else {
                // No else: if-expression has unit type
                ctx.tuple(vec![])
            }
        },

        HirExpr::Match { scrutinee, arms, span, .. } => {
            let scrut_tv = gen_expr(ctx, hir, *scrutinee);
            let result_tv = ctx.fresh();

            for arm in arms {
                // Pattern constrains scrutinee type
                gen_pat(ctx, hir, arm.pattern, scrut_tv);

                // Guard must be bool
                if let Some(guard) = arm.guard {
                    let guard_tv = gen_expr(ctx, hir, guard);
                    let bool_tv = ctx.named(ctx.resolver.builtin(Bool).unwrap(), vec![]);
                    ctx.equal(guard_tv, bool_tv, span.clone());
                }

                // Body must match result type
                let body_tv = gen_expr(ctx, hir, arm.body);
                ctx.equal(body_tv, result_tv, span.clone());
            }
            result_tv
        },

        HirExpr::Loop { body, .. } => {
            gen_block(ctx, hir, body);
            // Loop type is Never unless break provides a value (future)
            ctx.never()
        },

        HirExpr::Break { .. } | HirExpr::Continue { .. } => ctx.never(),

        HirExpr::Return { value, span, .. } => {
            if let Some(val) = value {
                let val_tv = gen_expr(ctx, hir, *val);
                ctx.coerce(val_tv, ctx.return_ty, id, span.clone());
            }
            ctx.never()
        },

        // === Assignment ===
        HirExpr::Assign { target, value, span, .. } => {
            let target_tv = gen_expr(ctx, hir, *target);
            let value_tv = gen_expr(ctx, hir, *value);
            ctx.coerce(value_tv, target_tv, id, span.clone());
            ctx.tuple(vec![]) // assignment returns unit
        },

        // === Closures ===
        HirExpr::Closure { params, body, .. } => {
            gen_closure(ctx, hir, params, body)
        },

        // === Aggregates ===
        HirExpr::Array { elements, span, .. } => {
            let elem_tv = ctx.fresh();
            for &e in elements {
                let e_tv = gen_expr(ctx, hir, e);
                ctx.equal(e_tv, elem_tv, span.clone());
            }
            // Result is an array literal — uses LiteralKind::Array
            // so it can default to DefaultArrayLiteralType[elem]
            let result = ctx.fresh_literal(LiteralKind::Array);
            // The element type is linked via the array's type arg
            // Solver handles: when result resolves to Array[T], equal(T, elem_tv)
            ctx.constraints.push(Constraint::ArrayLiteral {
                element: elem_tv, result, span: span.clone(),
            });
            result
        },

        HirExpr::Tuple { elements, span, .. } => {
            let elem_tvs: Vec<TyVar> = elements.iter()
                .map(|&e| gen_expr(ctx, hir, e))
                .collect();
            ctx.tuple(elem_tvs)
        },

        HirExpr::Error { .. } => ctx.report_error(InferError::from_hir_error(id)),
        // ... other variants ...
    };

    // Record the type for this expression
    ctx.expr_types.insert(id, tv);
    tv
}
```

### Statement Generation

```rust
fn gen_stmt(ctx: &mut InferCtx, hir: &HirBody, id: HirStmtId) {
    match &hir.stmts[id] {
        HirStmt::Let { local, ty, value, span, .. } => {
            let local_tv = if let Some(ty) = ty {
                // Annotated: convert HirTy → TyVar
                lower_hir_ty(ctx, ty)
            } else {
                // Unannotated: fresh TyVar, inferred from value
                ctx.fresh()
            };

            ctx.local_types.insert(*local, local_tv);

            if let Some(val) = value {
                let val_tv = gen_expr(ctx, hir, *val);
                // Value flows to the binding (allows promotion)
                ctx.coerce(val_tv, local_tv, *val, span.clone());
            }
        },

        HirStmt::Expr { expr, .. } => {
            gen_expr(ctx, hir, *expr);
        },

        // ... Deinit, etc. ...
    }
}
```

### Pattern Generation

```rust
/// Generate constraints for a pattern, given the type of the scrutinee.
fn gen_pat(ctx: &mut InferCtx, hir: &HirBody, pat_id: HirPatId, scrutinee_tv: TyVar) {
    match &hir.pats[pat_id] {
        HirPat::Wildcard { .. } => {
            // No constraint — matches anything
        },

        HirPat::Binding { local, .. } => {
            // Bind local to the scrutinee type
            ctx.local_types.insert(*local, scrutinee_tv);
        },

        HirPat::Literal { value, span, .. } => {
            // Literal must match scrutinee type
            let lit_tv = literal_to_tyvar(ctx, value);
            ctx.equal(lit_tv, scrutinee_tv, span.clone());
        },

        HirPat::Tuple { elements, span, .. } => {
            // Scrutinee must be a tuple with matching arity
            let elem_tvs: Vec<TyVar> = elements.iter()
                .map(|_| ctx.fresh())
                .collect();
            let tuple_tv = ctx.tuple(elem_tvs.clone());
            ctx.equal(scrutinee_tv, tuple_tv, span.clone());

            for (elem_pat, elem_tv) in elements.iter().zip(elem_tvs) {
                gen_pat(ctx, hir, *elem_pat, elem_tv);
            }
        },

        HirPat::Variant { entity, args, span, .. } => {
            // Resolved enum case: look up case parameter types
            // and constrain scrutinee to be the parent enum type
            gen_variant_pat(ctx, hir, *entity, args, scrutinee_tv, span);
        },

        HirPat::ImplicitVariant { name, args, span, .. } => {
            // Like Implicit constraint: resolve against scrutinee type
            gen_implicit_variant_pat(ctx, hir, name, args, scrutinee_tv, span);
        },

        HirPat::Struct { entity, fields, span, .. } => {
            gen_struct_pat(ctx, hir, *entity, fields, scrutinee_tv, span);
        },

        HirPat::Or { alternatives, .. } => {
            for &alt in alternatives {
                gen_pat(ctx, hir, alt, scrutinee_tv);
            }
        },

        HirPat::Error { .. } => { /* swallow */ },
        // ... Range, etc. ...
    }
}
```

### Generic Instantiation

Called when referencing a generic entity (function, method, struct init):

```rust
/// Instantiate a generic entity: create fresh TyVars for type params,
/// emit where clause constraints, return the instantiated type.
fn instantiate(ctx: &mut InferCtx, entity: Entity) -> TyVar {
    let type_params = /* read TypeParameter children from entity */;

    if type_params.is_empty() {
        // Non-generic: just return the entity's declared type
        return entity_type(ctx, entity);
    }

    // Fresh TyVar per type parameter
    let fresh: Vec<TyVar> = type_params.iter()
        .map(|_| ctx.fresh())
        .collect();

    // Emit where clause constraints
    for clause in ctx.resolver.where_clauses(entity) {
        match clause {
            WhereClause::Bound { param, protocol } => {
                let idx = type_params.iter().position(|&p| p == param).unwrap();
                ctx.conforms(fresh[idx], protocol, span);
            },
            WhereClause::TypeEquality { left, right } => {
                let left_tv = substitute_type_ref(ctx, &left, &type_params, &fresh);
                let right_tv = substitute_type_ref(ctx, &right, &type_params, &fresh);
                ctx.equal(left_tv, right_tv, span);
            },
        }
    }

    // Build the instantiated type (substituting type params with fresh TyVars)
    build_instantiated_type(ctx, entity, &type_params, &fresh)
}
```

### Closure Generation

```rust
fn gen_closure(
    ctx: &mut InferCtx,
    hir: &HirBody,
    params: &[ClosureParam],
    body: &HirBlock,
) -> TyVar {
    // Fresh TyVars for each param (may have type annotation)
    let param_tvs: Vec<TyVar> = params.iter().map(|p| {
        let tv = if let Some(ty) = &p.ty {
            lower_hir_ty(ctx, ty)
        } else {
            ctx.fresh()
        };
        ctx.local_types.insert(p.local, tv);
        tv
    }).collect();

    // Infer body
    let body_tv = gen_block(ctx, hir, body);

    // Build function type
    ctx.function(param_tvs, body_tv)
}
```

Bidirectional inference handles the rest: when the closure is passed as an argument, `Coerce` unifies the function type with the expected parameter type, flowing type information back into the closure's param TyVars.

---

## Solver (`solver.rs`)

### Main Loop

```rust
fn solve(ctx: &mut InferCtx) {
    // Phase 1: Main solving — iterate until fixpoint
    loop {
        let progress = solve_round(ctx);
        if !progress { break; }
    }

    // Phase 2: Apply literal defaults for unconstrained literals
    apply_literal_defaults(ctx);

    // Phase 3: Solve again with defaults applied
    loop {
        let progress = solve_round(ctx);
        if !progress { break; }
    }

    // Phase 4: Default remaining unconstrained TyVars to Never
    // (handles cases like unused generic params in error paths)
    apply_never_defaults(ctx);
}

fn solve_round(ctx: &mut InferCtx) -> bool {
    let mut progress = false;
    let constraints = std::mem::take(&mut ctx.constraints);

    for constraint in constraints {
        match try_solve(ctx, constraint) {
            SolveResult::Solved => progress = true,
            SolveResult::Deferred(c) => ctx.constraints.push(c),
            SolveResult::Error(err) => {
                ctx.report_error(err);
                progress = true; // error counts as progress (removes constraint)
            },
        }
    }

    progress
}

enum SolveResult {
    Solved,
    Deferred(Constraint),
    Error(InferError),
}
```

### Constraint Dispatch

```rust
fn try_solve(ctx: &mut InferCtx, c: Constraint) -> SolveResult {
    match c {
        Constraint::Equal { a, b, span } => solve_equal(ctx, a, b, span),
        Constraint::Coerce { from, to, expr, span } => solve_coerce(ctx, from, to, expr, span),
        Constraint::Conforms { ty, protocol, span } => solve_conforms(ctx, ty, protocol, span),
        Constraint::Associated { container, name, result, span } =>
            solve_associated(ctx, container, &name, result, span),
        Constraint::Member { receiver, name, args, result, expr, span } =>
            solve_member(ctx, receiver, &name, args, result, expr, span),
        Constraint::Implicit { expected, name, args, result, expr, span } =>
            solve_implicit(ctx, expected, &name, args, result, expr, span),
    }
}
```

### Solving Each Constraint

**Equal:**
```rust
fn solve_equal(ctx: &mut InferCtx, a: TyVar, b: TyVar, span: Span) -> SolveResult {
    match unify(ctx, a, b) {
        Ok(()) => SolveResult::Solved,
        Err(UnifyError::Mismatch) => SolveResult::Error(InferError::TypeMismatch { a, b, span }),
        Err(UnifyError::OccursCheck) => SolveResult::Error(InferError::InfiniteType { span }),
    }
}
```

**Coerce:**
```rust
fn solve_coerce(ctx: &mut InferCtx, from: TyVar, to: TyVar, expr: HirExprId, span: Span)
    -> SolveResult
{
    // Try unification first (handles the common case)
    match unify(ctx, from, to) {
        Ok(()) => return SolveResult::Solved,
        Err(UnifyError::LiteralGuard) => {
            // Literal couldn't unify with target (doesn't conform to ExpressibleBy*).
            // Fall through to promotion check.
        },
        Err(UnifyError::Mismatch) => {
            // Types don't match structurally. Try promotion.
        },
        Err(UnifyError::OccursCheck) => {
            return SolveResult::Error(InferError::InfiniteType { span });
        },
    }

    // Promotion: check if to-type conforms to FromValue[from-type]
    let from_resolved = ctx.resolve(from);
    let to_resolved = ctx.resolve(to);

    if !ctx.is_concrete(from_resolved) || !ctx.is_concrete(to_resolved) {
        // Can't check promotion yet — defer
        return SolveResult::Deferred(Constraint::Coerce { from, to, expr, span });
    }

    if let Some(promotion) = ctx.resolver.check_promotion(
        ctx.slot(from_resolved),
        ctx.slot(to_resolved),
    ) {
        ctx.promotions.insert(expr, promotion);
        SolveResult::Solved
    } else {
        SolveResult::Error(InferError::TypeMismatch { a: from, b: to, span })
    }
}
```

**Conforms:**
```rust
fn solve_conforms(ctx: &mut InferCtx, ty: TyVar, protocol: Entity, span: Span)
    -> SolveResult
{
    let resolved = ctx.resolve(ty);
    match ctx.slot(resolved) {
        TySlot::Unresolved { .. } => SolveResult::Deferred(Constraint::Conforms { ty, protocol, span }),
        TySlot::Resolved(TyKind::Error) => SolveResult::Solved, // swallow
        TySlot::Resolved(kind) => {
            if ctx.resolver.conforms_to(kind, protocol) {
                SolveResult::Solved
            } else {
                SolveResult::Error(InferError::DoesNotConform { ty, protocol, span })
            }
        },
        _ => unreachable!(), // resolve() follows redirects
    }
}
```

**Associated:**
```rust
fn solve_associated(ctx: &mut InferCtx, container: TyVar, name: &str,
                    result: TyVar, span: Span) -> SolveResult
{
    let resolved = ctx.resolve(container);
    if !ctx.is_concrete(resolved) {
        return SolveResult::Deferred(Constraint::Associated {
            container, name: name.to_string(), result, span,
        });
    }

    let kind = ctx.slot(resolved);
    if let TySlot::Resolved(TyKind::Error) = kind {
        return SolveResult::Solved;
    }

    match ctx.resolver.resolve_associated_type(kind.as_concrete(), name) {
        Some(assoc) => {
            let assoc_tv = lower_resolved_type(ctx, &assoc);
            solve_equal(ctx, assoc_tv, result, span)
        },
        None => SolveResult::Error(InferError::NoAssociatedType {
            container, name: name.to_string(), span,
        }),
    }
}
```

**Member (the big one):**
```rust
fn solve_member(ctx: &mut InferCtx, receiver: TyVar, name: &str,
                args: Vec<CallArg>, result: TyVar, expr: HirExprId, span: Span)
    -> SolveResult
{
    let resolved = ctx.resolve(receiver);
    if !ctx.is_concrete(resolved) {
        return SolveResult::Deferred(Constraint::Member {
            receiver, name: name.to_string(), args, result, expr, span,
        });
    }

    let recv_kind = ctx.slot(resolved);
    if let TySlot::Resolved(TyKind::Error) = recv_kind {
        return SolveResult::Solved;
    }

    // Resolve the member via the type resolver
    let resolution = match ctx.resolver.resolve_member(recv_kind.as_concrete(), name, &args) {
        Ok(res) => res,
        Err(MemberError::NotFound) =>
            return SolveResult::Error(InferError::NoMember {
                receiver, name: name.to_string(), span,
            }),
        Err(MemberError::Ambiguous(_)) =>
            return SolveResult::Error(InferError::AmbiguousMember {
                receiver, name: name.to_string(), span,
            }),
        Err(MemberError::NotVisible) =>
            return SolveResult::Error(InferError::MemberNotVisible {
                receiver, name: name.to_string(), span,
            }),
    };

    // Record the resolved entity
    ctx.resolutions.insert(expr, resolution.entity);

    // Instantiate the member's type parameters
    let fresh_params: Vec<TyVar> = resolution.type_params.iter()
        .map(|_| ctx.fresh())
        .collect();

    // Record inferred type args for this call
    if !fresh_params.is_empty() {
        ctx.type_args.insert(expr, fresh_params.clone());
    }

    // Emit where clause constraints from the resolved member
    for clause in &resolution.where_clauses {
        match clause {
            WhereClause::Bound { param, protocol } => {
                let idx = resolution.type_params.iter()
                    .position(|&p| p == *param).unwrap();
                ctx.conforms(fresh_params[idx], *protocol, span.clone());
            },
            WhereClause::TypeEquality { left, right } => {
                let l = substitute_assoc(ctx, left, &resolution, &fresh_params, resolved);
                let r = substitute_type(ctx, right, &resolution, &fresh_params, resolved);
                ctx.equal(l, r, span.clone());
            },
        }
    }

    // Equate argument types with parameter types
    let param_types = substitute_types(ctx, &resolution.param_types,
                                        &resolution, &fresh_params, resolved);
    for (arg, param_tv) in args.iter().zip(param_types) {
        ctx.coerce(arg.ty, param_tv, expr, span.clone());
    }

    // Equate result with return type
    let ret_tv = substitute_type(ctx, &resolution.return_type,
                                  &resolution, &fresh_params, resolved);
    ctx.equal(result, ret_tv, span.clone());

    SolveResult::Solved
}
```

---

## Unification (`unify.rs`)

```rust
enum UnifyError {
    Mismatch,
    LiteralGuard,  // literal TyVar couldn't adopt target type
    OccursCheck,
}

fn unify(ctx: &mut InferCtx, a: TyVar, b: TyVar) -> Result<(), UnifyError> {
    let a = ctx.resolve(a);
    let b = ctx.resolve(b);

    // Same TyVar — trivially equal
    if a == b { return Ok(()); }

    match (ctx.slot(a), ctx.slot(b)) {
        // Error poisons: silently absorb
        (TySlot::Resolved(TyKind::Error), _) |
        (_, TySlot::Resolved(TyKind::Error)) => Ok(()),

        // Never (bottom type): unifies with anything.
        // But if the other side is Unresolved, DON'T bind it —
        // let other constraints resolve the Infer first.
        (TySlot::Resolved(TyKind::Never), TySlot::Unresolved { .. }) |
        (TySlot::Unresolved { .. }, TySlot::Resolved(TyKind::Never)) => Ok(()),
        (TySlot::Resolved(TyKind::Never), _) |
        (_, TySlot::Resolved(TyKind::Never)) => Ok(()),

        // Both unresolved: link them.
        // If one is a literal, keep the literal marker.
        (TySlot::Unresolved { literal: lit_a }, TySlot::Unresolved { literal: lit_b }) => {
            let merged_literal = lit_a.or(*lit_b);
            ctx.types[a.0] = TySlot::Redirect(b);
            if merged_literal.is_some() {
                ctx.types[b.0] = TySlot::Unresolved { literal: merged_literal };
            }
            Ok(())
        },

        // Unresolved (non-literal) + Concrete: bind
        (TySlot::Unresolved { literal: None }, _) => {
            occurs_check(ctx, a, b)?;
            ctx.types[a.0] = TySlot::Redirect(b);
            Ok(())
        },
        (_, TySlot::Unresolved { literal: None }) => {
            occurs_check(ctx, b, a)?;
            ctx.types[b.0] = TySlot::Redirect(a);
            Ok(())
        },

        // Literal TyVar + Concrete: guard — check ExpressibleBy* conformance
        (TySlot::Unresolved { literal: Some(lit) }, TySlot::Resolved(kind)) => {
            if conforms_to_literal_protocol(ctx, kind, *lit) {
                occurs_check(ctx, a, b)?;
                ctx.types[a.0] = TySlot::Redirect(b);
                Ok(())
            } else {
                Err(UnifyError::LiteralGuard)
            }
        },
        (TySlot::Resolved(kind), TySlot::Unresolved { literal: Some(lit) }) => {
            if conforms_to_literal_protocol(ctx, kind, *lit) {
                occurs_check(ctx, b, a)?;
                ctx.types[b.0] = TySlot::Redirect(a);
                Ok(())
            } else {
                Err(UnifyError::LiteralGuard)
            }
        },

        // Both concrete: structural unification
        (TySlot::Resolved(kind_a), TySlot::Resolved(kind_b)) => {
            unify_concrete(ctx, kind_a, kind_b)
        },

        // Redirect should be resolved by resolve()
        _ => unreachable!(),
    }
}

fn unify_concrete(ctx: &mut InferCtx, a: &TyKind, b: &TyKind) -> Result<(), UnifyError> {
    match (a, b) {
        // Named types: same entity + unify type args
        (TyKind::Named { entity: ea, args: aa },
         TyKind::Named { entity: eb, args: ab }) => {
            if ea != eb || aa.len() != ab.len() { return Err(UnifyError::Mismatch); }
            for (&a, &b) in aa.iter().zip(ab) {
                unify(ctx, a, b)?;
            }
            Ok(())
        },

        // Tuples: same arity + unify elements
        (TyKind::Tuple(ea), TyKind::Tuple(eb)) => {
            if ea.len() != eb.len() { return Err(UnifyError::Mismatch); }
            for (&a, &b) in ea.iter().zip(eb) {
                unify(ctx, a, b)?;
            }
            Ok(())
        },

        // Functions: same arity + unify params + unify return
        (TyKind::Function { params: pa, ret: ra },
         TyKind::Function { params: pb, ret: rb }) => {
            if pa.len() != pb.len() { return Err(UnifyError::Mismatch); }
            for (&a, &b) in pa.iter().zip(pb) {
                unify(ctx, a, b)?;
            }
            unify(ctx, *ra, *rb)
        },

        // Type params: must be the same entity
        (TyKind::Param { entity: a }, TyKind::Param { entity: b }) => {
            if a == b { Ok(()) } else { Err(UnifyError::Mismatch) }
        },

        _ => Err(UnifyError::Mismatch),
    }
}

/// Occurs check: ensure tv doesn't appear in target (prevents infinite types).
fn occurs_check(ctx: &InferCtx, tv: TyVar, target: TyVar) -> Result<(), UnifyError> {
    let target = ctx.resolve(target);
    if tv == target { return Err(UnifyError::OccursCheck); }
    match ctx.slot(target) {
        TySlot::Resolved(TyKind::Named { args, .. }) => {
            for &arg in args { occurs_check(ctx, tv, arg)?; }
            Ok(())
        },
        TySlot::Resolved(TyKind::Tuple(elems)) => {
            for &e in elems { occurs_check(ctx, tv, e)?; }
            Ok(())
        },
        TySlot::Resolved(TyKind::Function { params, ret }) => {
            for &p in params { occurs_check(ctx, tv, p)?; }
            occurs_check(ctx, tv, *ret)
        },
        _ => Ok(()),
    }
}

/// Check if a concrete type conforms to the literal's ExpressibleBy* protocol.
fn conforms_to_literal_protocol(ctx: &InferCtx, ty: &TyKind, lit: LiteralKind) -> bool {
    let feature = match lit {
        LiteralKind::Integer    => BuiltinFeature::ExpressibleByIntegerLiteral,
        LiteralKind::Float      => BuiltinFeature::ExpressibleByFloatLiteral,
        LiteralKind::String     => BuiltinFeature::ExpressibleByStringLiteral,
        LiteralKind::Bool       => BuiltinFeature::ExpressibleByBoolLiteral,
        LiteralKind::Char       => BuiltinFeature::ExpressibleByCharLiteral,
        LiteralKind::Null       => BuiltinFeature::ExpressibleByNullLiteral,
        LiteralKind::Array      => BuiltinFeature::ExpressibleByArrayLiteral,
        LiteralKind::Dictionary => BuiltinFeature::ExpressibleByDictionaryLiteral,
    };
    let Some(protocol) = ctx.resolver.builtin(feature) else { return false; };
    ctx.resolver.conforms_to(ty, protocol)
}
```

---

## Literal Defaults

```rust
fn apply_literal_defaults(ctx: &mut InferCtx) {
    let mut applied = false;
    for idx in 0..ctx.types.len() {
        let tv = TyVar(idx as u32);
        let resolved = ctx.resolve(tv);
        let TySlot::Unresolved { literal: Some(lit) } = ctx.slot(resolved) else { continue; };

        let feature = match lit {
            LiteralKind::Integer    => BuiltinFeature::DefaultIntegerLiteralType,
            LiteralKind::Float      => BuiltinFeature::DefaultFloatLiteralType,
            LiteralKind::String     => BuiltinFeature::DefaultStringLiteralType,
            LiteralKind::Bool       => BuiltinFeature::DefaultBooleanLiteralType,
            LiteralKind::Char       => BuiltinFeature::DefaultCharLiteralType,
            LiteralKind::Null       => BuiltinFeature::DefaultNullLiteralType,
            LiteralKind::Array      => BuiltinFeature::DefaultArrayLiteralType,
            LiteralKind::Dictionary => BuiltinFeature::DefaultDictionaryLiteralType,
        };

        if let Some(alias_entity) = ctx.resolver.builtin(feature) {
            // Instantiate the alias — for generic aliases (Array[T], Dict[K,V]),
            // this creates fresh TyVars that get constrained by existing
            // element-type constraints
            let default_tv = instantiate_alias(ctx, alias_entity);

            // Clear the literal marker and bind
            ctx.types[resolved.0] = TySlot::Redirect(default_tv);
            applied = true;
        }
    }
}
```

---

## Error Recovery (`error.rs`)

```rust
/// Type inference error. Each variant maps to a user-facing diagnostic.
enum InferError {
    /// τ₁ ≠ τ₂ — types don't match
    TypeMismatch { expected: TyVar, got: TyVar, span: Span },

    /// τ doesn't conform to Protocol
    DoesNotConform { ty: TyVar, protocol: Entity, span: Span },

    /// No member 'name' on type τ
    NoMember { receiver: TyVar, name: String, span: Span },

    /// Ambiguous member — multiple candidates
    AmbiguousMember { receiver: TyVar, name: String, span: Span },

    /// Private member
    MemberNotVisible { receiver: TyVar, name: String, span: Span },

    /// No associated type 'name' on container
    NoAssociatedType { container: TyVar, name: String, span: Span },

    /// Infinite type (occurs check failure)
    InfiniteType { span: Span },

    /// Error propagated from HIR (HirExpr::Error, HirPat::Error, etc.)
    FromHir { span: Span },

    /// Implicit member .name not found on expected type
    ImplicitMemberNotFound { expected: TyVar, name: String, span: Span },
}
```

Every path that produces an error calls `ctx.report_error(err)`, which:
1. Pushes the error to `ctx.errors`
2. Returns a TyVar with `TyKind::Error`

This guarantees every Error TyVar in the system has a corresponding diagnostic (rustc's ErrorGuaranteed pattern).

---

## Output (`result.rs`)

```rust
/// Result of type inference for a single body.
struct TypedBody {
    /// Type of every expression.
    expr_types: HashMap<HirExprId, ResolvedTy>,

    /// Resolved entity for MethodCall/Field expressions.
    /// Used by codegen to know which function to call.
    resolutions: HashMap<HirExprId, Entity>,

    /// Promotion info for expressions that need wrapping.
    /// Codegen inserts FromValue.from() calls at these sites.
    promotions: HashMap<HirExprId, PromotionInfo>,

    /// Inferred type arguments for generic calls.
    type_args: HashMap<HirExprId, Vec<ResolvedTy>>,

    /// Errors accumulated during inference.
    errors: Vec<InferError>,
}

/// A fully resolved type (no TyVars).
enum ResolvedTy {
    Named { entity: Entity, args: Vec<ResolvedTy> },
    Param { entity: Entity },
    Tuple(Vec<ResolvedTy>),
    Function { params: Vec<ResolvedTy>, ret: Box<ResolvedTy> },
    Never,
    Error,
}

struct PromotionInfo {
    /// The FromValue.from() method entity to call.
    method: Entity,
    /// Target type (what we're promoting to).
    target: ResolvedTy,
}
```

### Building TypedBody from InferCtx

After solving completes, resolve all TyVars to concrete types:

```rust
fn build_result(ctx: &InferCtx) -> TypedBody {
    let expr_types = ctx.expr_types.iter()
        .map(|(&id, &tv)| (id, resolve_to_concrete(ctx, tv)))
        .collect();

    let resolutions = ctx.resolutions.clone();

    let promotions = ctx.promotions.iter()
        .map(|(&id, info)| (id, resolve_promotion(ctx, info)))
        .collect();

    let type_args = ctx.type_args.iter()
        .map(|(&id, tvs)| (id, tvs.iter().map(|&tv| resolve_to_concrete(ctx, tv)).collect()))
        .collect();

    TypedBody { expr_types, resolutions, promotions, type_args, errors: ctx.errors.clone() }
}
```

---

## Query Definition (`lib.rs`)

```rust
/// Query: infer types for a function/init/getter body.
///
/// Reads HirBody (from LowerBody query), generates constraints,
/// solves them, and returns a TypedBody with resolved types.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct InferBody {
    pub entity: Entity,
    pub root: Entity,
}

impl QueryFn for InferBody {
    type Output = Option<TypedBody>;

    fn execute(&self, ctx: &QueryContext<'_>) -> Option<TypedBody> {
        // Get the HIR body
        let hir = ctx.query(LowerBody {
            entity: self.entity,
            root: self.root,
        })?;

        // Create the type resolver
        let resolver = WorldResolver {
            ctx, root: self.root, owner: self.entity,
        };

        // Create inference context
        let mut infer_ctx = InferCtx::new(&resolver, self.entity, self.root);

        // Generate constraints from HIR
        generate(&mut infer_ctx, &hir);

        // Solve
        solve(&mut infer_ctx);

        // Build output
        Some(build_result(&infer_ctx))
    }
}
```

---

## Design Principles

1. **Single Responsibility**: Each file does one thing.
   - `ty.rs`: type representation
   - `constraint.rs`: constraint definitions
   - `generate.rs`: walks HIR, emits constraints
   - `solver.rs`: fixpoint loop
   - `unify.rs`: structural unification
   - `resolve.rs`: world queries

2. **DRY**: One `Member` constraint replaces 5+ old resolution methods.
   One `Coerce` replaces separate Equal + Promotable. Literal defaults
   are uniform through builtin aliases.

3. **Open/Closed**: `TypeResolver` trait allows swapping implementations
   (real world vs test mocks) without changing the solver.

4. **Dependency Inversion**: Solver depends on `TypeResolver` trait,
   not on concrete `QueryContext`. The `WorldResolver` implementation
   is in `resolve.rs`, separate from solver logic.

5. **Error Recovery**: ErrorGuaranteed pattern — every `TyKind::Error`
   has a corresponding diagnostic. Error types silently absorb all
   constraints, preventing cascading errors.

6. **Bidirectional**: `Coerce` at value boundaries enables expected-type
   propagation. Closure param types flow backwards from call sites.
   Generic type args flow backwards from argument types.

7. **No Deferred Expressions**: HIR already has the right shape.
   Method names are strings, field names are strings. The solver
   resolves them to entities and records in `resolutions` table.

## Estimated Size

| File | Lines |
|------|-------|
| lib.rs | ~50 |
| ty.rs | ~100 |
| constraint.rs | ~60 |
| ctx.rs | ~150 |
| generate.rs | ~400 |
| solver.rs | ~250 |
| unify.rs | ~150 |
| resolve.rs | ~300 |
| result.rs | ~100 |
| error.rs | ~80 |
| **Total** | **~1640** |

vs old system: ~5000+ lines across 8 files.
