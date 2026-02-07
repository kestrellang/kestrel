# Compound Assignment Operators Implementation Plan

## Test Strategy

### Test Categories
1. **Basic arithmetic compound assignments**: `+=`, `-=`, `*=`, `/=`, `%=`
2. **Bitwise compound assignments**: `&=`, `|=`, `^=`
3. **Shift compound assignments**: `<<=`, `>>=`
4. **Mutability validation**: Ensure target is mutable
5. **Type checking**: Verify protocol conformance
6. **Invalid targets**: Literals, immutable bindings
7. **Complex targets**: Field access, subscript access

### Key Behaviors to Verify
- `x += 1` desugars to `x.addAssign(1)`
- Compound assignment returns `()` (unit type)
- Target must be mutable (`var`, not `let`)
- Chaining is disallowed due to `()` return type

---

## Implementation Phases

### Phase 0: Tests (First!)
**Files**: `lib/kestrel-test-suite/tests/compound_assignment.rs`

- [ ] Basic compound assignment with integers
- [ ] Compound assignment with different types (Float, custom)
- [ ] Mutability errors (assigning to `let`)
- [ ] Invalid targets (literals)
- [ ] Field access compound assignment
- [ ] Verify `()` return type prevents chaining

---

### Phase 1: Lexer
**Files**: `lib/kestrel-lexer/src/lib.rs`

Add 10 new compound assignment tokens (must come BEFORE their single-char equivalents for correct matching):

- [ ] `PlusEquals` (`+=`)
- [ ] `MinusEquals` (`-=`)
- [ ] `StarEquals` (`*=`)
- [ ] `SlashEquals` (`/=`)
- [ ] `PercentEquals` (`%=`)
- [ ] `AmpersandEquals` (`&=`)
- [ ] `PipeEquals` (`|=`)
- [ ] `CaretEquals` (`^=`)
- [ ] `LessLessEquals` (`<<=`)
- [ ] `GreaterGreaterEquals` (`>>=`)

---

### Phase 2: Syntax Tree
**Files**: `lib/kestrel-syntax-tree/src/lib.rs`

- [ ] Add `SyntaxKind` variants for all 10 compound operator tokens
- [ ] Add to `From<Token> for SyntaxKind` implementation
- [ ] Add to `kind_from_raw()` match statement (CRITICAL!)
- [ ] Add `ExprCompoundAssignment` syntax node kind

---

### Phase 3: Parser
**Files**: `lib/kestrel-parser/src/expr/mod.rs`

- [ ] Add compound operator tokens to assignment parsing (after binary, alongside `=`)
- [ ] Create `ExprVariant::CompoundAssignment` with `lhs`, `operator`, `operator_span`, `rhs`
- [ ] Add `emit_compound_assignment_expr()` function
- [ ] Update `emit_expr_variant()` to handle compound assignment

Parser structure:
```rust
// In assignment section (around line 1891)
// Check for compound assignment OR regular assignment
let compound_or_assign = skip_trivia()
    .ignore_then(
        just(Token::PlusEquals).to(CompoundOp::Add)
            .or(just(Token::MinusEquals).to(CompoundOp::Sub))
            // ... etc
            .map_with(|op, e| (Some(op), to_kestrel_span(e.span())))
            .or(just(Token::Equals).map_with(|_, e| (None, to_kestrel_span(e.span()))))
    )
    .then(expr.clone())
    .or_not();
```

---

### Phase 4: Semantic Tree
**Files**: `lib/kestrel-semantic-tree/src/operators.rs`

- [ ] Add `CompoundOp` enum with all 10 operators
- [ ] Add `method_name()` mapping to protocol methods
- [ ] Add `symbol()` for error messages

```rust
pub enum CompoundOp {
    Add,      // += -> addAssign
    Sub,      // -= -> subtractAssign
    Mul,      // *= -> multiplyAssign
    Div,      // /= -> divideAssign
    Rem,      // %= -> modAssign
    BitAnd,   // &= -> bitwiseAndAssign
    BitOr,    // |= -> bitwiseOrAssign
    BitXor,   // ^= -> bitwiseXorAssign
    Shl,      // <<= -> shiftLeftAssign
    Shr,      // >>= -> shiftRightAssign
}
```

---

### Phase 5: Semantic Expression
**Files**: `lib/kestrel-semantic-tree/src/expr/mod.rs` (or similar)

- [ ] Add `ExprKind::CompoundAssignment` variant
- [ ] Include: target expression, compound operator, value expression
- [ ] Or: Reuse assignment with optional operator field

---

### Phase 6: Body Resolver
**Files**: `lib/kestrel-semantic-tree-binder/src/body_resolver/expressions.rs`

- [ ] Add `resolve_compound_assignment_expression()` function
- [ ] Desugar to method call: `target.{method_name}(value)`
- [ ] Return type is `()` (unit)
- [ ] Handle `SyntaxKind::ExprCompoundAssignment` in expression resolution

```rust
fn resolve_compound_assignment_expression(
    node: &SyntaxNode,
    ctx: &mut BodyResolutionContext
) -> Expression {
    let span = get_node_span(node, ctx.file_id);

    // Extract operator, target, value from syntax
    let op = extract_compound_op(node);
    let target = resolve_expression(&target_node, ctx);
    let value = resolve_expression(&value_node, ctx);

    // Desugar to method call
    let method_name = op.method_name();
    let arg = CallArgument::unlabeled(value.clone(), value.span.clone());

    // Create method call with Unit return type
    Expression::deferred_method_call(
        target,
        method_name.to_string(),
        vec![arg],
        Ty::unit(span.clone()),  // Returns ()
        span,
    )
}
```

---

### Phase 7: Validation (if needed)
**Files**: `lib/kestrel-semantic-analyzers/src/analyzers/assignment_validation/mod.rs`

- [ ] Extend `AssignmentValidationAnalyzer` to handle compound assignments
- [ ] Reuse existing target validation (mutable check)
- [ ] May not need changes if body resolver emits method calls that get validated elsewhere

---

### Phase 8: Standard Library - Default Implementations
**Files**: `lang/std/core/assign.ks`

Uncomment and enable default protocol implementations:

- [ ] `extend Add[Rhs]: AddAssign[Rhs] where Add[Rhs].Output = Self`
- [ ] `extend Subtract[Rhs]: SubtractAssign[Rhs] where Subtract[Rhs].Output = Self`
- [ ] `extend Multiply[Rhs]: MultiplyAssign[Rhs] where Multiply[Rhs].Output = Self`
- [ ] `extend Divide[Rhs]: DivideAssign[Rhs] where Divide[Rhs].Output = Self`
- [ ] `extend Modulo[Rhs]: ModuloAssign[Rhs] where Modulo[Rhs].Output = Self`
- [ ] `extend BitwiseAnd[Rhs]: BitwiseAndAssign[Rhs] where BitwiseAnd[Rhs].Output = Self`
- [ ] `extend BitwiseOr[Rhs]: BitwiseOrAssign[Rhs] where BitwiseOrAssign[Rhs].Output = Self`
- [ ] `extend BitwiseXor[Rhs]: BitwiseXorAssign[Rhs] where BitwiseXor[Rhs].Output = Self`
- [ ] `extend LeftShift[Rhs]: LeftShiftAssign[Rhs] where LeftShift[Rhs].Output = Self`
- [ ] `extend RightShift[Rhs]: RightShiftAssign[Rhs] where RightShift[Rhs].Output = Self`

Note: Protocol names are `Addable`, `Subtractable`, etc. - verify exact names in codebase.

---

## Verification

After each phase:
```bash
cargo test
```

Final verification:
- [ ] All tests pass: `cargo test`
- [ ] Linting clean: `cargo clippy`
- [ ] Formatted: `cargo fmt`

---

## File Modification Summary

| File | Changes |
|------|---------|
| `kestrel-lexer/src/lib.rs` | Add 10 compound operator tokens |
| `kestrel-syntax-tree/src/lib.rs` | Add SyntaxKind variants + kind_from_raw |
| `kestrel-parser/src/expr/mod.rs` | Parse compound assignment |
| `kestrel-semantic-tree/src/operators.rs` | Add CompoundOp enum |
| `kestrel-semantic-tree/src/expr/mod.rs` | Add expression variant (if needed) |
| `kestrel-semantic-tree-binder/src/body_resolver/expressions.rs` | Resolve compound assignment |
| `kestrel-semantic-analyzers/src/analyzers/assignment_validation/mod.rs` | Extend validation (if needed) |
| `lang/std/core/assign.ks` | Enable default implementations |
| `kestrel-test-suite/tests/compound_assignment.rs` | Add comprehensive tests |
