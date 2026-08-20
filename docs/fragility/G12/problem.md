# G12 — six analyzers answered "does this diverge?", three different ways, and two of them were wrong

`high` · `false accept` (missed `E002`) + `false reject` (spurious `E500`) ·
crate: `kestrel-analyze`
(`src/body/{dead_code,exhaustive_return,definite_assignment,move_tracking,guard,initializer}.rs`)

G8/G9/G10 unified *"can this loop exit?"*. It did not unify the question that
asks it: **"does this construct ever fall through to what comes next?"** Five
analyzers carried a private copy of that rule and a sixth consulted no types at
all.

## The inventory

| analyzer | `Loop` mechanism | Never-typed leaf? | `HirStmt::Let`? |
| --- | --- | --- | --- |
| `guard.rs` | `!contains_break_for` | yes — **checked first**, before the structural arms | no |
| `exhaustive_return.rs` | `!contains_break_for`, three-way `ReturnState` | yes, as the leaf fallback | **yes** |
| `dead_code.rs` | `!contains_break_for` + two dead helpers | **no — consulted no types at all** | no |
| `definite_assignment.rs` | `body_state.diverged && !contains_break_for` | yes, trailing, unguarded | no |
| `move_tracking.rs` | `body_state.diverged && !contains_break_for` | yes, trailing, **`Loop` excluded** | no |
| `initializer.rs` | `break_states.is_empty()` (reachability-aware) | yes, trailing, skipped via early `return` | n/a |

Three mechanisms for one fact, two of them wrong, and every copy disagreeing
with the others about which sub-questions it even asks.

## Live consequence 1 — `dead_code` misses `E002`

`dead_code` never read `TypedBody`, so a `-> !` call was an ordinary
`HirExpr::Call` and therefore, to it, an ordinary fall-through. Reproduced at
top level, inside a `while`, and after a `match` whose every arm diverges:

```kestrel
func topLevel() {
    fatalError("boom");
    let a: lang.i64 = 1;   // never executes — no warning emitted
}
```

Analyzer output before the fix: `(errors=0 warnings=0)`. After: one `E002` per
shape.

## Live consequence 2 — a false `E500`

The `body_state.diverged && !contains_break_for(...)` formula is provably wrong.
For `loop { doWork(); }` the body *completes* — `diverged` is false — so the
conjunction answers **"this loop does not diverge"** about an infinite loop.

`definite_assignment` and `initializer` never showed it because a trailing
Never-type check re-decided the same question and got it right by accident.
`move_tracking` explicitly excluded `HirExpr::Loop` from that check, so nothing
rescued it:

```kestrel
var h = Handle(fd: 42);
consume(h);
loop { doWork(); }
consume(h);            // error[E500]: use of moved value 'h'
```

Both diagnostics printed from the real CLI, about the same line: `E500` "value
used here after move" and `E002` "this code will never execute".

## The carve-outs' stated justification was false

Two comments — `exhaustive_return.rs:206-209` and `initializer.rs:611` — said
"every loop is typed `Never`, so we must skip the Never check". They are wrong:
`kestrel-type-infer`'s `generate.rs:590-605` unifies a loop's `break_tv` with
unit at every `break` that targets it, so `loop { break; }` is unit-typed. The
early returns those comments justified are still correct, for a different
reason: **structure decides, inference must not overrule it.**

Conversely, `Break`/`Continue`/`Return` expressions *are* unconditionally
`Never`-typed regardless of whether they stand in a valid position — which is
why `dead_code`'s `in_loop` parameter and its "labeled break might target a
non-enclosing loop" conservatism were never needed.
