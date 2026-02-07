# Optional and Result Type Promotion Implementation Plan (v2)

## Overview

Implement implicit type promotion for Optional and Result types using a **constraint-based approach**. A new `Promotable` constraint replaces `Equals` at assignment sites, trying unification first and falling back to `FromValue` conformance checking.

```
Binder              Constraint Gen         Solver              apply_solution
   │                     │                    │                      │
   ▼                     ▼                    ▼                      ▼
Normal AST          Emit Promotable      1. Try unify         If expr in
(no changes)        at assignment        2. Check FromValue   promotions map:
                    sites                3. Record if needed  wrap with from()
```

## Architecture

### Key Design Decisions

1. **Constraint-based, not AST-based**: No new `ExprKind` variant. Promotion is tracked via constraints and the solution.

2. **Promotable subsumes Equals**: At assignment sites, `Promotable` tries unification first, then falls back to promotion. This is more permissive than `Equals`.

3. **Solution tracks promotions by ExprId**: Following the existing `values: HashMap<ExprId, ValueResolution>` pattern, we add `promotions: HashMap<ExprId, PromotionInfo>`.

4. **apply_solution wraps expressions**: After processing children, check if the expression needs promotion wrapping.

### Constraint Comparison

| Aspect | Equals | Promotable |
|--------|--------|------------|
| Direction | Bidirectional | `from_ty` → `to_ty` |
| Fields | `a`, `b`, `span` | `from_ty`, `to_ty`, `expr_id`, `span` |
| Unification | Always attempts | Attempts first |
| Fallback | None (error) | Check `FromValue` conformance |
| Side effect | Updates type vars | May record promotion in solution |

---

## Phase 1: Standard Library

### 1.1 Add FromValue Protocol

**File**: `lang/std/core/error.ks` (after line 40, after `FromResidual`)

```kestrel
/// Protocol that enables value promotion.
/// Types conforming to FromValue can be constructed from a success value,
/// allowing implicit promotion from T to Optional[T] or Result[T, E].
@builtin(.FromValueProtocol)
protocol FromValue[Output] {
    /// Creates an instance from a success value.
    @builtin(.FromValueMethod)
    static func from(_ value: Output) -> Self
}
```

**Note**: Not `public` - internal to stdlib.

### 1.2 Optional Conformance

**File**: `lang/std/result/optional.ks` (after `FromResidual` extension)

```kestrel
/// FromValue extension enabling value promotion.
/// Allows: let x: Int? = 5
extend Optional[T]: FromValue[T] {
    static func from(_ value: T) -> Optional[T] {
        .Some(value)
    }
}
```

### 1.3 Result Conformance

**File**: `lang/std/result/result.ks` (after `FromResidual` extension)

```kestrel
/// FromValue extension enabling value promotion.
/// Allows: let r: Int throws Error = 42
extend Result[T, E]: FromValue[T] {
    static func from(_ value: T) -> Result[T, E] {
        .Ok(value)
    }
}
```

### 1.4 Import Updates

Add `FromValue` to imports in `optional.ks` and `result.ks`:
```kestrel
import std.core.(..., FromValue)
```

---

## Phase 2: Builtins Registry

**File**: `lib/kestrel-semantic-tree/src/builtins.rs`

### 2.1 Add Enum Variants

```rust
// After FromResidualMethod (around line 128):
FromValueProtocol,
FromValueMethod,
```

### 2.2 Add String Mappings

In `from_name()`:
```rust
"FromValueProtocol" => Some(Self::FromValueProtocol),
"FromValueMethod" => Some(Self::FromValueMethod),
```

In `name()`:
```rust
Self::FromValueProtocol => "FromValueProtocol",
Self::FromValueMethod => "FromValueMethod",
```

### 2.3 Add Definitions

In `definition()`:
```rust
Self::FromValueProtocol => BuiltinDefinition {
    feature: *self,
    kind: BuiltinKind::Protocol {
        implicit_conformance: false,
        must_be_marker: false,
        tuple_conformance_propagation: false,
        requires_fields_conform: false,
        disallow_enum_conformance: false,
    },
},
Self::FromValueMethod => BuiltinDefinition {
    feature: *self,
    kind: BuiltinKind::ProtocolMethod {
        protocol_feature: LanguageFeature::FromValueProtocol,
    },
},
```

---

## Phase 3: Solution Types

**File**: `lib/kestrel-semantic-type-inference/src/solution.rs`

### 3.1 Add PromotionInfo Struct

```rust
/// Information about a value promotion.
/// When an expression needs to be wrapped with FromValue.from(),
/// this records the target type and resolved method.
#[derive(Debug, Clone)]
pub struct PromotionInfo {
    /// The target type (e.g., Optional[Int])
    pub target_ty: Ty,
    /// The resolved FromValue.from method symbol
    pub from_method: SymbolId,
    /// Type substitutions for the method call
    pub substitutions: Substitutions,
}

impl PromotionInfo {
    pub fn new(target_ty: Ty, from_method: SymbolId, substitutions: Substitutions) -> Self {
        Self { target_ty, from_method, substitutions }
    }
}
```

### 3.2 Add Promotions Field to Solution

```rust
pub struct Solution {
    pub types: HashMap<TyId, Ty>,
    pub values: HashMap<ExprId, ValueResolution>,
    pub promotions: HashMap<ExprId, PromotionInfo>,  // NEW
    pub errors: Vec<InferenceError>,
}
```

### 3.3 Add Accessor Method

```rust
impl Solution {
    pub fn get_promotion(&self, expr_id: ExprId) -> Option<&PromotionInfo> {
        self.promotions.get(&expr_id)
    }
}
```

### 3.4 Update Constructor

Update `Solution::new()` and `Solution::with_errors()` to initialize `promotions: HashMap::new()`.

---

## Phase 4: Inference Context

**File**: `lib/kestrel-semantic-type-inference/src/context.rs`

### 4.1 Add Promotions Field

```rust
pub struct InferenceContext<'a> {
    // ... existing fields ...
    promotions: HashMap<ExprId, PromotionInfo>,  // NEW
}
```

### 4.2 Add Accessor Methods

```rust
impl<'a> InferenceContext<'a> {
    pub fn promotions_mut(&mut self) -> &mut HashMap<ExprId, PromotionInfo> {
        &mut self.promotions
    }
}
```

### 4.3 Update into_solution()

```rust
pub(crate) fn into_solution(self) -> Solution {
    Solution {
        types: self.substitutions,
        values: self.values,
        promotions: self.promotions,  // NEW
        errors: self.errors,
    }
}
```

---

## Phase 5: Constraint Definition

**File**: `lib/kestrel-semantic-type-inference/src/constraint.rs`

### 5.1 Add Promotable Variant

```rust
pub enum Constraint {
    // ... existing variants ...

    /// A value may be promoted to a target type.
    /// First tries unification. If that fails, checks if target
    /// conforms to FromValue[source]. Records promotion if so.
    Promotable {
        /// The source expression's type
        from_ty: TyId,
        /// The target type to assign to
        to_ty: TyId,
        /// The expression that may need wrapping
        expr_id: ExprId,
        /// Source location for errors
        span: Span,
    },
}
```

### 5.2 Update span() Method

```rust
impl Constraint {
    pub fn span(&self) -> &Span {
        match self {
            // ... existing cases ...
            Self::Promotable { span, .. } => span,
        }
    }
}
```

### 5.3 Add Constructor Helper

In context.rs:
```rust
impl<'a> InferenceContext<'a> {
    pub fn promotable(&mut self, from_ty: TyId, to_ty: TyId, expr_id: ExprId, span: Span) {
        self.push_constraint(Constraint::Promotable { from_ty, to_ty, expr_id, span });
    }
}
```

---

## Phase 6: Solver

**File**: `lib/kestrel-semantic-type-inference/src/solver.rs`

### 6.1 Add Handler in try_solve()

```rust
fn try_solve(ctx: &mut InferenceContext<'_>, constraint: &Constraint) -> Result<SolveResult, InferenceError> {
    match constraint {
        // ... existing cases ...
        Constraint::Promotable { from_ty, to_ty, expr_id, span } => {
            resolve_promotable(ctx, *from_ty, *to_ty, *expr_id, span.clone())
        }
    }
}
```

### 6.2 Implement resolve_promotable()

```rust
fn resolve_promotable(
    ctx: &mut InferenceContext<'_>,
    from_ty: TyId,
    to_ty: TyId,
    expr_id: ExprId,
    span: Span,
) -> Result<SolveResult, InferenceError> {
    let from = resolve_type(ctx, from_ty);
    let to = resolve_type(ctx, to_ty);

    // Defer if either type still has inference variables
    if has_unresolved_infer(&from) || has_unresolved_infer(&to) {
        return Ok(SolveResult::Deferred);
    }

    // 1. Try direct unification first
    if can_unify(ctx, &from, &to) {
        // Unify and we're done - no promotion needed
        return unify_types(ctx, from_ty, to_ty, span);
    }

    // 2. Check if target conforms to FromValue[source]
    if let Some(promo_info) = check_from_value_conformance(ctx, &to, &from) {
        // Record promotion for apply_solution
        ctx.promotions_mut().insert(expr_id, promo_info);
        return Ok(SolveResult::Solved);
    }

    // 3. Neither unification nor promotion worked
    Err(InferenceError::type_mismatch(to, from, span))
}

fn check_from_value_conformance(
    ctx: &InferenceContext<'_>,
    target_ty: &Ty,
    source_ty: &Ty,
) -> Option<PromotionInfo> {
    // Get the FromValueProtocol symbol
    let from_value_protocol = ctx.oracle().builtin_protocol(LanguageFeature::FromValueProtocol)?;

    // Check if target conforms to FromValue[source]
    // This requires building a ProtocolRef with source as the type argument
    let protocol_ref = ProtocolRef::with_args(from_value_protocol, vec![source_ty.clone()]);

    if !ctx.oracle().conforms_to(target_ty, &protocol_ref) {
        return None;
    }

    // Resolve the from() method
    let from_method = ctx.oracle().builtin_method(LanguageFeature::FromValueMethod)?;

    // Build substitutions (Output -> source_ty)
    let substitutions = /* ... build substitutions ... */;

    Some(PromotionInfo::new(target_ty.clone(), from_method, substitutions))
}
```

### 6.3 Helper: can_unify()

```rust
/// Check if two types can unify without actually unifying them.
fn can_unify(ctx: &InferenceContext<'_>, a: &Ty, b: &Ty) -> bool {
    // Types are equal
    if a == b {
        return true;
    }

    // One is a subtype of the other (Never, Error types, etc.)
    if ctx.oracle().is_subtype(a, b) || ctx.oracle().is_subtype(b, a) {
        return true;
    }

    // Struct-to-protocol coercion
    if matches!(b.kind(), TyKind::Protocol { .. }) && ctx.oracle().conforms_to(a, b) {
        return true;
    }

    false
}
```

---

## Phase 7: Constraint Generator

**File**: `lib/kestrel-semantic-type-inference/src/constraint_generator.rs`

Replace `ctx.equate()` with `ctx.promotable()` at assignment sites.

### 7.1 Variable Declarations (lines ~56-68)

```rust
StatementKind::Binding { pattern, value } => {
    generate_pattern_constraints(ctx, pattern);

    if let Some(init) = value {
        generate_expression_constraints(ctx, init);
        ctx.register_type(&init.ty);
        // CHANGED: Use promotable instead of equate
        ctx.promotable(init.ty.id(), pattern.ty.id(), init.id, stmt.span.clone());
    }
}
```

### 7.2 Return Statements (lines ~812-831)

```rust
ExprKind::Return { value } => {
    if let Some(val) = value {
        generate_expression_constraints(ctx, val);

        if let Some(ret_ty) = ctx.return_type().cloned() {
            ctx.register_type(&ret_ty);
            ctx.register_type(&val.ty);
            // CHANGED: Use promotable instead of equate
            ctx.promotable(val.ty.id(), ret_ty.id(), val.id, val.span.clone());
        }
    }
    // ... handle no-value return ...
}
```

### 7.3 Assignments (lines ~724-728)

```rust
ExprKind::Assignment { target, value } => {
    generate_expression_constraints(ctx, target);
    generate_expression_constraints(ctx, value);
    // CHANGED: Use promotable instead of equate
    ctx.promotable(value.ty.id(), target.ty.id(), value.id, expr.span.clone());
}
```

### 7.4 Function Call Arguments (in call handling ~525-591)

```rust
// For each argument/parameter pair:
for (arg, param_ty) in arguments.iter().zip(params.iter()) {
    ctx.register_type(&arg.value.ty);
    ctx.register_type(param_ty);
    // CHANGED: Use promotable instead of equate
    ctx.promotable(arg.value.ty.id(), param_ty.id(), arg.value.id, arg.span.clone());
}
```

### 7.5 If Expression Branches (lines ~731-775)

```rust
// For then branch:
if let Some(then_val) = then_value {
    generate_expression_constraints(ctx, then_val);
    if else_branch.is_some() {
        // CHANGED: Use promotable instead of equate
        ctx.promotable(then_val.ty.id(), expr.ty.id(), then_val.id, then_val.span.clone());
    }
}

// For else branch:
if let Some(else_val) = value {
    generate_expression_constraints(ctx, else_val);
    // CHANGED: Use promotable instead of equate
    ctx.promotable(else_val.ty.id(), expr.ty.id(), else_val.id, else_val.span.clone());
}
```

### 7.6 Match Expression Arms (lines ~1025-1060)

```rust
for arm in arms {
    generate_expression_constraints(ctx, &arm.body);
    ctx.register_type(&arm.body.ty);
    // CHANGED: Use promotable instead of equate
    ctx.promotable(arm.body.ty.id(), expr.ty.id(), arm.body.id, arm.body.span.clone());
}
```

### 7.7 Yield Expression (lines ~42-52)

```rust
if let Some(yield_expr) = block.yield_expr() {
    generate_expression_constraints(ctx, yield_expr);

    if let Some(ret_ty) = return_type {
        ctx.register_type(ret_ty);
        ctx.register_type(&yield_expr.ty);
        // CHANGED: Use promotable instead of equate
        ctx.promotable(yield_expr.ty.id(), ret_ty.id(), yield_expr.id, yield_expr.span.clone());
    }
}
```

---

## Phase 8: apply_solution

**File**: `lib/kestrel-semantic-type-inference/src/apply.rs`

### 8.1 Check for Promotions After Processing

Modify `apply_to_expression()` to check for promotions after processing the expression:

```rust
fn apply_to_expression(expr: &Expression, solution: &Solution) -> Expression {
    let resolved_ty = resolve_type(&expr.ty, solution);

    // Process the expression as normal
    let processed = apply_to_expression_inner(expr, &resolved_ty, solution);

    // Check if this expression needs promotion wrapping
    if let Some(promo) = solution.get_promotion(expr.id) {
        return wrap_with_promotion(processed, promo);
    }

    processed
}
```

### 8.2 Implement wrap_with_promotion()

```rust
fn wrap_with_promotion(inner: Expression, promo: &PromotionInfo) -> Expression {
    let span = inner.span.clone();

    // Build: TargetType.from(inner)
    // 1. Create TypeRef for target type
    let type_ref = Expression::new(
        ExprKind::TypeRef {
            ty: promo.target_ty.clone(),
        },
        Ty::metatype(promo.target_ty.clone(), span.clone()),
        span.clone(),
        false,
    );

    // 2. Create MethodRef for from()
    let method_ref = Expression::new(
        ExprKind::MethodRef {
            receiver: Box::new(type_ref),
            method_ids: vec![promo.from_method],
            method_name: "from".to_string(),
        },
        /* function type */,
        span.clone(),
        false,
    );

    // 3. Create Call expression
    Expression::new(
        ExprKind::Call {
            callee: Box::new(method_ref),
            arguments: vec![CallArgument::unlabeled(inner)],
            substitutions: promo.substitutions.clone(),
        },
        promo.target_ty.clone(),
        span,
        false,
    )
}
```

---

## Phase 9: Type Checking (Error Messages)

**File**: `lib/kestrel-semantic-analyzers/src/analyzers/type_check/mod.rs`

The solver already produces `InferenceError::type_mismatch` when neither unification nor promotion works. Verify error messages are clear:

```
Error: Cannot assign `Int` to `String?`
  - `Int` is not assignable to `String?`
  - `String?` does not conform to `FromValue[Int]`
```

---

## Phase 10: Tests

**File**: `lib/kestrel-test-suite/tests/types/type_promotion.rs` (new)

### Basic Tests

```rust
#[test]
fn test_optional_promotion_in_variable() {
    // let x: Int? = 5
    assert_compiles!("let x: Int? = 5");
}

#[test]
fn test_result_promotion_in_variable() {
    // let r: Int throws Error = 42
    assert_compiles!("let r: Int throws Error = 42");
}

#[test]
fn test_optional_promotion_in_return() {
    assert_compiles!("fn f() -> Int? { return 5 }");
}

#[test]
fn test_result_promotion_in_return() {
    assert_compiles!("fn f() -> Int throws Error { return 42 }");
}

#[test]
fn test_promotion_in_assignment() {
    assert_compiles!("var x: Int? = nil; x = 5");
}

#[test]
fn test_promotion_in_function_arg() {
    assert_compiles!("fn f(x: Int?) {}; f(5)");
}

#[test]
fn test_promotion_in_if_branch() {
    assert_compiles!("let x: Int? = if true { 5 } else { nil }");
}

#[test]
fn test_no_promotion_without_annotation() {
    // x should be Int, not Int?
    assert_type_is!("let x = 5", "x", "Int");
}
```

### Negative Tests

```rust
#[test]
fn test_nested_optional_no_promotion() {
    // Int?? does not conform to FromValue[Int]
    assert_error!("let x: Int?? = 5", "type mismatch");
}

#[test]
fn test_incompatible_type_no_promotion() {
    // String? does not conform to FromValue[Int]
    assert_error!("let x: String? = 5", "type mismatch");
}
```

### Generic Tests

```rust
#[test]
fn test_generic_function_promotion() {
    assert_compiles!("fn wrap[T](x: T) -> T? { return x }");
}
```

---

## Files Modified Summary

| File | Changes |
|------|---------|
| `lang/std/core/error.ks` | Add `FromValue` protocol |
| `lang/std/result/optional.ks` | Add `FromValue[T]` conformance, import |
| `lang/std/result/result.ks` | Add `FromValue[T]` conformance, import |
| `lib/kestrel-semantic-tree/src/builtins.rs` | Add `FromValueProtocol`, `FromValueMethod` |
| `lib/kestrel-semantic-type-inference/src/solution.rs` | Add `PromotionInfo`, `promotions` field |
| `lib/kestrel-semantic-type-inference/src/context.rs` | Add `promotions` field, `promotable()` helper |
| `lib/kestrel-semantic-type-inference/src/constraint.rs` | Add `Promotable` variant |
| `lib/kestrel-semantic-type-inference/src/solver.rs` | Add `resolve_promotable()` handler |
| `lib/kestrel-semantic-type-inference/src/constraint_generator.rs` | Replace `equate` with `promotable` at 7 sites |
| `lib/kestrel-semantic-type-inference/src/apply.rs` | Add promotion wrapping logic |
| `lib/kestrel-test-suite/tests/types/type_promotion.rs` | New test file |

---

## Implementation Order

1. **Phase 1-2**: Stdlib + Builtins (can be done together)
2. **Phase 3-5**: Solution types + Context + Constraint definition
3. **Phase 6**: Solver (the core logic)
4. **Phase 7**: Constraint generator (change equate → promotable)
5. **Phase 8**: apply_solution (wrap expressions)
6. **Phase 9-10**: Error messages + Tests

---

## Potential Gotchas

1. **Unification vs Promotion order**: Must try unification FIRST. If we check promotion first, we might unnecessarily wrap expressions that could unify.

2. **Deferred constraints**: If types have inference variables, defer. Don't try to check conformance on `Infer` types.

3. **Nested optionals**: `Int?? = 5` should fail because `Optional[Optional[Int]]` conforms to `FromValue[Optional[Int]]`, not `FromValue[Int]`.

4. **Never type**: `Never` is assignable to everything - unification should handle this before promotion check.

5. **Error type propagation**: `Error` types should unify with anything to prevent cascading errors.

6. **Expression ID stability**: The expr_id used in the constraint must match the expr_id checked in apply_solution. Verify IDs are preserved through constraint solving.
