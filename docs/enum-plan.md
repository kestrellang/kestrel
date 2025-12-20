# Enum Implementation Plan (Revised)

Based on the design in [enums.md](./enums.md) and lessons learned from the first implementation attempt.

## Key Principles

1. **Parser captures everything** - The parser must capture all syntax as distinct nodes
2. **Body resolver translates, doesn't resolve** - Syntax → Semantic tree only, no type lookups
3. **Type inference handles shorthand** - `.Case` resolution happens during constraint solving
4. **No `expected_type` threading** - Remove bidirectional type context from body resolver

---

## Problems with Previous Implementation

### 1. Parser: Broken `indirect` Recognition
**File:** `lib/kestrel-parser/src/enum_decl/mod.rs:133-140`

```rust
let indirect_parser = identifier()
    .try_map(|span, _| {
        // PROBLEM: Accepts ANY identifier, not just "indirect"
        Ok(span)
    })
    .or_not();
```

**Fix:** Check identifier text is "indirect" using source string.

### 2. Body Resolver: `expected_type` Field
**File:** `lib/kestrel-semantic-tree-binder/src/body_resolver/context.rs:57`

```rust
pub expected_type: Option<kestrel_semantic_tree::ty::Ty>,
```

**Problem:** This conflates body resolution with type inference. Every site that creates `BodyResolutionContext` must handle it.

**Fix:** Remove entirely. Shorthand resolution moves to type inference.

### 3. Body Resolver: Type Lookups During Resolution
**File:** `lib/kestrel-semantic-tree-binder/src/body_resolver/expressions.rs:1288-1481`

`resolve_implicit_member_access` does:
- Looks up expected type from context
- Queries enum cases by name
- Validates labels and arity
- Creates fully-resolved expressions

**Problem:** Body resolver should only translate syntax → semantic. Type lookups belong in type inference.

**Fix:** Create `ExpressionKind::ImplicitMemberAccess` with unresolved type (`Ty::infer()`). Type inference resolves it.

### 4. Type Inference: Missing Enum Unification
**File:** `lib/kestrel-semantic-type-inference/src/solver.rs`

The solver has unification for `Struct` and `Protocol` but missing proper `Enum` unification case. Lines 113-126 added a semantic equality check, but there's no full unification case with substitution handling.

---

## Revised Architecture

### Phase Overview

| Phase | Component | Responsibility |
|-------|-----------|----------------|
| 1 | Lexer | `Enum`, `Case` tokens |
| 2 | Syntax Tree | All `SyntaxKind` variants |
| 3 | Parser | Full syntax capture including `.Case` |
| 4 | Semantic Symbols | `EnumSymbol`, `EnumCaseSymbol` |
| 5 | Type System | `TyKind::Enum`, `TyKind::ImplicitMember` |
| 6 | Builder | Symbol creation |
| 7 | Binder | Generics + parameter type resolution |
| 8 | Expression Types | `ExpressionKind::ImplicitMemberAccess` |
| 9 | Body Resolver | Simple syntax→semantic translation |
| 10 | Type Inference | Resolve implicit members, unify enums |
| 11 | Analyzers | Recursion, duplicates |
| 12 | Tests | Full coverage |

---

## Phase 1: Lexer ✓

**File:** `lib/kestrel-lexer/src/lib.rs`

Already done:
- `Enum` token
- `Case` token
- `indirect` stays as identifier (contextual keyword)

---

## Phase 2: Syntax Tree ✓

**File:** `lib/kestrel-syntax-tree/src/lib.rs`

Already done:
- `EnumDeclaration`
- `EnumBody`
- `EnumCaseDeclaration`
- `EnumCaseParameter`
- `EnumCaseParameterList`
- `IndirectModifier`
- `ExprImplicitMemberAccess`

---

## Phase 3: Parser

**Files:**
- `lib/kestrel-parser/src/enum_decl/mod.rs`
- `lib/kestrel-parser/src/common/data.rs`
- `lib/kestrel-parser/src/common/emitters.rs`

### 3.1 Fix `indirect` Parsing

Change `indirect_parser` to validate the identifier text:

```rust
fn indirect_modifier_parser(
    source: &str,
) -> impl Parser<Token, Option<Span>, Error = Simple<Token>> + Clone {
    identifier()
        .try_map(move |span: Span, _| {
            let text = &source[span.range()];
            if text == "indirect" {
                Ok(Some(span))
            } else {
                Err(Simple::expected_input_found(span.range(), vec![], None))
            }
        })
        .or(empty().map(|_| None))
}
```

### 3.2 Emit `EnumCaseParameterList`

Ensure parameters are wrapped in `EnumCaseParameterList` node:

```rust
// In emit_enum_case:
if let Some((lparen, params, rparen)) = &data.parameters {
    sink.start_node(SyntaxKind::EnumCaseParameterList);
    // emit params...
    sink.finish_node();
}
```

### 3.3 Implicit Member Access Parser

**File:** `lib/kestrel-parser/src/expressions/mod.rs`

The `.Case` and `.Case(args)` syntax must be parsed as `ExprImplicitMemberAccess`:

```rust
// Primary expression parser should include:
fn implicit_member_access_parser() -> impl Parser<...> {
    token(Token::Dot)
        .then(identifier())  // Case name
        .then(argument_list_parser().or_not())  // Optional (args)
        .map(|((dot, name), args)| ImplicitMemberAccessData {
            dot_span: dot,
            member_span: name,
            arguments: args,
        })
}
```

Emit as `ExprImplicitMemberAccess` node containing:
- `Dot` token
- `Name` node (the case name)
- Optional `ArgumentList` node

---

## Phase 4: Semantic Symbols ✓

**Files:**
- `lib/kestrel-semantic-tree/src/symbol/enum_symbol.rs`
- `lib/kestrel-semantic-tree/src/symbol/enum_case.rs`

Already done. Structure is correct.

---

## Phase 5: Type System

**File:** `lib/kestrel-semantic-tree/src/ty/kind.rs`

### 5.1 Existing `TyKind::Enum` ✓

```rust
Enum {
    symbol: Arc<EnumSymbol>,
    substitutions: Substitutions,
}
```

### 5.2 Add `TyKind::ImplicitMember` (NEW)

Add a new type kind for unresolved implicit member access:

```rust
/// Implicit member access pending resolution
/// Used for `.Case` syntax before type inference
ImplicitMember {
    /// The member name (case name)
    member_name: String,
    /// Arguments if provided (types are resolved but target is unknown)
    arguments: Vec<ImplicitMemberArg>,
}

/// An argument to an implicit member
pub struct ImplicitMemberArg {
    pub label: Option<String>,
    pub value_ty: Ty,
}
```

This type acts as a placeholder until type inference can determine the expected enum type.

---

## Phase 6: Builder ✓

**Files:**
- `lib/kestrel-semantic-tree-builder/src/builders/enum.rs`
- `lib/kestrel-semantic-tree-builder/src/builders/enum_case.rs`

Already done. Structure is correct.

---

## Phase 7: Binder ✓

**Files:**
- `lib/kestrel-semantic-tree-binder/src/binders/enum.rs`
- `lib/kestrel-semantic-tree-binder/src/binders/enum_case.rs`

Already done:
- `EnumBinder` resolves generics
- `EnumCaseBinder` resolves parameters and adds `CallableBehavior`

---

## Phase 8: Expression Types

**File:** `lib/kestrel-semantic-tree/src/expr.rs`

### 8.1 Existing `ExprKind::EnumCase` ✓

```rust
EnumCase { case_id: SymbolId }
```

Used for resolved enum case references (result of `Color.Red` after resolution).

### 8.2 Add `ExprKind::ImplicitMemberAccess` (NEW)

```rust
/// Implicit member access: `.Case` or `.Case(args)`
/// The target type is unknown at body resolution time.
/// Type inference will resolve this to EnumCase or Call.
ImplicitMemberAccess {
    member_name: String,
    arguments: Option<Vec<CallArgument>>,
}
```

---

## Phase 9: Body Resolver

**Key Change:** Remove `expected_type` from `BodyResolutionContext`.

### 9.1 Remove `expected_type` Field

**File:** `lib/kestrel-semantic-tree-binder/src/body_resolver/context.rs`

Delete line 57:
```rust
// DELETE: pub expected_type: Option<kestrel_semantic_tree::ty::Ty>,
```

Remove from all struct literals in:
- `context.rs` (line 80, 235)
- `function.rs` (line 163)
- `initializer.rs` (line 139)

### 9.2 Remove Expected Type Threading

**File:** `lib/kestrel-semantic-tree-binder/src/body_resolver/statements.rs:96-106`

Replace:
```rust
// OLD: Threading expected type
let old_expected = ctx.expected_type.take();
ctx.expected_type = ty.clone();
let expr = resolve_expression(&expr_node, ctx);
ctx.expected_type = old_expected;
```

With:
```rust
// NEW: Just resolve expression normally
let expr = resolve_expression(&expr_node, ctx);
```

### 9.3 Simplify Implicit Member Access Resolution

**File:** `lib/kestrel-semantic-tree-binder/src/body_resolver/expressions.rs`

Replace the complex `resolve_implicit_member_access` (lines 1288-1481) with:

```rust
fn resolve_implicit_member_access(node: &SyntaxNode, ctx: &mut BodyResolutionContext) -> Expression {
    let span = get_node_span(node, ctx.file_id);

    // Extract member name
    let member_name = node
        .children()
        .find(|c| c.kind() == SyntaxKind::Name)
        .and_then(|n| extract_identifier_from_name(&n))
        .unwrap_or_else(|| "?".to_string());

    // Extract arguments if present
    let arguments = node
        .children()
        .find(|c| c.kind() == SyntaxKind::ArgumentList)
        .map(|arg_list| resolve_argument_list(&arg_list, ctx));

    // Create unresolved implicit member access expression
    // Type inference will resolve this later
    Expression::implicit_member_access(member_name, arguments, span)
}
```

The expression has type `Ty::infer()`. Type inference will:
1. Unify with expected type
2. Look up the enum case
3. Validate arguments
4. Rewrite to resolved form

---

## Phase 10: Type Inference

**Files:**
- `lib/kestrel-semantic-type-inference/src/constraint.rs`
- `lib/kestrel-semantic-type-inference/src/constraint_generator.rs`
- `lib/kestrel-semantic-type-inference/src/solver.rs`

### 10.1 Add `ImplicitMember` Constraint

**File:** `lib/kestrel-semantic-type-inference/src/constraint.rs`

```rust
pub enum Constraint {
    // ... existing ...

    /// Resolve an implicit member access
    /// When expected type is known, resolves `.Case` to an enum case
    ImplicitMember {
        /// The expression's type variable
        expr_ty: TyId,
        /// The member name
        member_name: String,
        /// Argument types (if any)
        arguments: Vec<ImplicitMemberArg>,
        /// Expression ID for rewriting
        expr_id: ExprId,
        /// Span for error reporting
        span: Span,
    },
}
```

### 10.2 Generate Constraints for Implicit Members

**File:** `lib/kestrel-semantic-type-inference/src/constraint_generator.rs`

When visiting `ExprKind::ImplicitMemberAccess`:

```rust
ExprKind::ImplicitMemberAccess { member_name, arguments } => {
    // Register the expression's type as an inference variable
    let expr_ty = expr.ty.clone();
    ctx.register_type(&expr_ty);

    // Create constraint to resolve the implicit member
    ctx.add_constraint(Constraint::ImplicitMember {
        expr_ty: expr_ty.id(),
        member_name: member_name.clone(),
        arguments: /* convert arguments */,
        expr_id: expr.id,
        span: expr.span.clone(),
    });
}
```

### 10.3 Solve Implicit Member Constraints

**File:** `lib/kestrel-semantic-type-inference/src/solver.rs`

Add case in `try_solve`:

```rust
Constraint::ImplicitMember { expr_ty, member_name, arguments, expr_id, span } => {
    resolve_implicit_member(ctx, *expr_ty, member_name, arguments, *expr_id, span)
}
```

Implementation:

```rust
fn resolve_implicit_member(
    ctx: &mut InferenceContext<'_>,
    expr_ty: TyId,
    member_name: &str,
    arguments: &[ImplicitMemberArg],
    expr_id: ExprId,
    span: &Span,
) -> Result<SolveResult, InferenceError> {
    let resolved_ty = resolve_type(ctx, expr_ty);

    // If the type is still Infer, defer
    if matches!(resolved_ty.kind(), TyKind::Infer) {
        return Ok(SolveResult::Deferred);
    }

    // Must be an enum type
    let TyKind::Enum { symbol: enum_symbol, substitutions } = resolved_ty.kind() else {
        return Err(InferenceError::not_an_enum(resolved_ty.clone(), span.clone()));
    };

    // Look up the case
    let cases = enum_symbol.cases();
    let case = cases.iter()
        .find(|c| c.metadata().name().value == member_name);

    let Some(case) = case else {
        let available = cases.iter().map(|c| c.metadata().name().value.clone()).collect();
        return Err(InferenceError::unknown_enum_case(
            member_name.to_string(),
            enum_symbol.metadata().name().value.clone(),
            available,
            span.clone(),
        ));
    };

    // Validate arguments match case parameters
    let callable = case.metadata().get_behavior::<CallableBehavior>();

    match (callable, arguments.is_empty()) {
        (None, true) => {
            // Simple case, no args expected, none provided - OK
            // Record value resolution
            ctx.values_mut().insert(expr_id, ValueResolution::enum_case(case.metadata().id()));
            Ok(SolveResult::Solved)
        }
        (None, false) => {
            // Simple case but args provided
            Err(InferenceError::enum_case_arity(member_name.to_string(), 0, arguments.len(), span.clone()))
        }
        (Some(cb), true) if cb.parameters().is_empty() => {
            // Case with no params, no args - OK
            ctx.values_mut().insert(expr_id, ValueResolution::enum_case(case.metadata().id()));
            Ok(SolveResult::Solved)
        }
        (Some(cb), true) => {
            // Case expects args, none provided
            Err(InferenceError::enum_case_arity(member_name.to_string(), cb.parameters().len(), 0, span.clone()))
        }
        (Some(cb), false) => {
            let params = cb.parameters();
            if params.len() != arguments.len() {
                return Err(InferenceError::enum_case_arity(
                    member_name.to_string(),
                    params.len(),
                    arguments.len(),
                    span.clone()
                ));
            }

            // Validate labels
            for (i, (arg, param)) in arguments.iter().zip(params.iter()).enumerate() {
                let expected_label = param.label.as_ref().map(|l| l.value.as_str());
                if arg.label.as_deref() != expected_label {
                    return Err(InferenceError::enum_case_label(
                        member_name.to_string(),
                        expected_label.map(|s| s.to_string()),
                        arg.label.clone(),
                        i,
                        span.clone(),
                    ));
                }

                // Equate argument type with parameter type (with substitutions)
                let param_ty = param.ty.apply_substitutions(substitutions);
                ctx.equate(arg.value_ty.id(), param_ty.id(), span.clone());
            }

            // Record value resolution as a call to the enum case
            ctx.values_mut().insert(expr_id, ValueResolution::enum_case_call(case.metadata().id()));
            Ok(SolveResult::Solved)
        }
    }
}
```

### 10.4 Add Full Enum Unification

Add proper `TyKind::Enum` case in `unify` (similar to `Struct` case):

```rust
(
    TyKind::Enum {
        symbol: sym_a,
        substitutions: subs_a,
    },
    TyKind::Enum {
        symbol: sym_b,
        substitutions: subs_b,
    },
) => {
    use semantic_tree::symbol::Symbol;
    use kestrel_semantic_tree::language::KestrelLanguage;

    let id_a = Symbol::<KestrelLanguage>::metadata(sym_a.as_ref()).id();
    let id_b = Symbol::<KestrelLanguage>::metadata(sym_b.as_ref()).id();

    if id_a != id_b {
        return Err(InferenceError::type_mismatch(ty_a.clone(), ty_b.clone(), span.clone()));
    }

    // Unify substitutions by key
    for (key, sub_a) in subs_a.iter() {
        if let Some(sub_b) = subs_b.get(*key) {
            ctx.equate(sub_a.id(), sub_b.id(), span.clone());
        } else {
            return Err(InferenceError::type_mismatch(ty_a.clone(), ty_b.clone(), span.clone()));
        }
    }
    for (key, _) in subs_b.iter() {
        if !subs_a.contains(*key) {
            return Err(InferenceError::type_mismatch(ty_a.clone(), ty_b.clone(), span.clone()));
        }
    }
    Ok(SolveResult::Solved)
}
```

---

## Phase 11: Analyzers

**New file:** `lib/kestrel-semantic-analyzers/src/analyzers/enum_validation/mod.rs`

### 11.1 Recursive Enum Analyzer

Check that recursive enums have `indirect`:

```rust
pub struct RecursiveEnumAnalyzer;

impl Analyzer for RecursiveEnumAnalyzer {
    fn analyze(&mut self, model: &SemanticModel, ctx: &mut AnalysisContext) {
        // Walk all EnumSymbols
        // For each case, check if any parameter type references the enum
        // If recursive and not indirect, emit E0404
    }
}
```

### 11.2 Duplicate Case Analyzer

Check for duplicate case names:

```rust
pub struct DuplicateCaseAnalyzer;

impl Analyzer for DuplicateCaseAnalyzer {
    fn analyze(&mut self, model: &SemanticModel, ctx: &mut AnalysisContext) {
        // Walk all EnumSymbols
        // Check for duplicate case names
        // Emit E0405 if found
    }
}
```

---

## Phase 12: Tests

**File:** `lib/kestrel-test-suite/tests/declarations/enums.rs`

Test categories (already exists, verify coverage):

1. Declaration tests
2. Associated value tests
3. Generic enum tests
4. Nested enum tests
5. Simple instantiation tests
6. Instantiation with args tests
7. Shorthand instantiation tests
8. Recursive enum tests
9. Error case tests

---

## Files to Clean Up

When starting over, these files contain enum-related code to remove/rewrite:

| File | What to Remove/Change |
|------|----------------------|
| `body_resolver/context.rs` | Remove `expected_type` field |
| `body_resolver/statements.rs` | Remove expected type threading |
| `body_resolver/expressions.rs` | Replace `resolve_implicit_member_access` |
| `diagnostics/enum_errors.rs` | Keep but may need adjustment |
| `solver.rs` | Add proper enum unification |
| `constraint.rs` | Add `ImplicitMember` constraint |
| `constraint_generator.rs` | Handle `ImplicitMemberAccess` |

---

## Implementation Order

| Step | Description | Complexity |
|------|-------------|------------|
| 1 | Fix `indirect` parser | Low |
| 2 | Add `ExprKind::ImplicitMemberAccess` | Low |
| 3 | Simplify body resolver | Medium |
| 4 | Remove `expected_type` from context | Medium (many sites) |
| 5 | Add `ImplicitMember` constraint | Medium |
| 6 | Implement constraint solver | High |
| 7 | Add enum unification | Low |
| 8 | Write analyzers | Medium |
| 9 | Run tests, fix issues | Variable |

---

## Error Codes

| Code | Error | Where Emitted |
|------|-------|---------------|
| E0401 | Unknown enum case | Type inference |
| E0402 | Missing/wrong label | Type inference |
| E0403 | Cannot infer type for shorthand | Type inference |
| E0404 | Recursive requires `indirect` | Analyzer |
| E0405 | Duplicate case name | Analyzer |
| E0406 | Duplicate label in case | Binder |
| E0407 | Type mismatch | Type inference |
| E0408 | Wrong arity | Type inference |
