# Boolean Guard

**Status**: In Progress
**Issue**: [#25](https://github.com/kestrellang/kestrel/issues/25)
**Target**: 0.16

## Summary

Add a boolean guard form alongside the existing `guard let`:

```kestrel
guard isValid else {
    return
}
```

This complements `guard let`, which binds an optional:

```kestrel
guard let value = maybeValue else {
    return
}
```

The two forms can be mixed in a single condition chain:

```kestrel
guard isActive, let .Some(user) = fetchUser(id) else {
    return .error
}
```

## Motivation

Many early-return checks are plain boolean conditions, not optional unwraps. Today you'd write:

```kestrel
if not isValid {
    return
}
```

A boolean `guard` makes intent clearer — "this must hold for the rest of the scope" — and keeps the style consistent when a function mixes optional unwraps and boolean preconditions.

## Syntax

```
guard <condition> else { <diverging-block> }
guard <condition>, <condition>, ... else { <diverging-block> }
```

Where each `<condition>` is either:
- A boolean expression (or any expression whose type is Bool / conforms to `BooleanConditional`)
- A `let` binding: `let <pattern> = <expr>`

This is the same condition grammar as `if` and `while`.

## Semantics

- The condition expression must be Bool or conform to `BooleanConditional` — same rule as `if`/`while` conditions.
- The `else` block must diverge (return, break, continue, or throw) — same rule as `guard let`.
- Boolean conditions introduce no new bindings. `let` conditions bind into the enclosing scope as before.
- Multiple conditions are ANDed: all must hold for execution to continue past the guard.

## Desugaring

Boolean guard reuses the exact same desugaring path as `guard let`. The AST `GuardLet` node's condition list already supports `IfCondition::Expr` — only the parser restricted the first condition to `let`.

```
guard <expr> else { <else_body> }
```

Lowers to:

```
// HIR
HirExpr::If {
    condition: lower(<expr>),        // boolean expression directly
    then_body: { },                  // empty
    else_body: { <else_body> },      // must diverge
}
```

Compare with `guard let .Some(x) = opt else { ... }`:

```
// HIR
HirExpr::If {
    condition: Match {               // desugared pattern match → Bool
        scrutinee: opt,
        arms: [.Some(x) => true, _ => false]
    },
    then_body: { },
    else_body: { <else_body> },
}
```

## Pipeline Trace

| Stage | What happens | Changes needed |
|-------|-------------|----------------|
| **Parser** | `guard` token → parse condition chain → `else { block }` | Allow `IfCondition::Expr` as first condition (was `let`-only) |
| **AST** | `AstStmt::GuardLet { conditions: Vec<IfCondition>, else_body }` | None — `IfCondition::Expr` already exists |
| **HIR lowering** | `lower_if_conditions` handles `Expr` → direct expression lowering | None |
| **Type inference** | `is_guard_let_if` skips type-equating else block | None — condition-type agnostic |
| **Condition check (E101)** | Validates condition is Bool/BooleanConditional | Fix label: "guard" not "if" for guard-originated conditions |
| **Divergence (E003)** | Verifies else block diverges | None — condition-type agnostic |
| **Exhaustiveness (E309)** | Warns on irrefutable patterns | None — only fires for `let` conditions that produce Match nodes |
| **MIR/codegen** | Operates on desugared `HirExpr::If` | None |

## Diagnostics

- **E003** (`guard_let_else_must_diverge`): Fires when the else block doesn't diverge. Applies to both boolean and let guards.
- **E101** (`condition_not_bool`): Fires when condition isn't Bool/BooleanConditional. Already covers guard via `HirExpr::If` iteration; label updated to say "guard" for guard-originated expressions.
- **E309** (`irrefutable_guard_let`): Only fires for `let` conditions. Boolean guards have no pattern to check.

## Examples

### Basic boolean guard

```kestrel
func process(x: Int) -> Int {
    guard x > 0 else {
        return 0
    }
    x * 2
}
```

### Mixed chain

```kestrel
func process(flag: Bool, opt: Optional[User]) -> Result {
    guard flag, let .Some(user) = opt else {
        return .error
    }
    user.activate()
}
```

### Non-diverging else (E003 error)

```kestrel
func bad(x: Int) -> Int {
    guard x > 0 else {
        0  // ERROR: guard else block must diverge
    }
    x
}
```
