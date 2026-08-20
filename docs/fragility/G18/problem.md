# G18 — every implicit condition branched on the condition's raw bits, never calling `boolValue()`

`high` · `silent miscompile` · crates: `kestrel-mir-lower`
(`src/body/control.rs`, `src/body/pattern.rs`, `src/body/mod.rs`),
`kestrel-mir` (`src/lib.rs`, `src/ty_query.rs`), `kestrel-hir`
(`src/builtin.rs`), `lang/std/core/bool.ks`

`if x` on a `BooleanConditional` conformer type-checked, compiled, ran, and
took the **wrong branch**. No diagnostic, no ICE, no crash — just the other arm.

## The repro

```kestrel
module Test

struct Inverted: BooleanConditional {
    var v: lang.i64

    func boolValue() -> lang.i1 {
        lang.i64_eq(self.v, 0)     // NONZERO payload is FALSE
    }
}

@main
func main() -> lang.i64 {
    let a = Inverted(v: 200);
    if a { 7 } else { 9 }          // boolValue() is false -> 9
}
```

```
$ ./target/release/kestrel build inverted.ks -o inverted && ./inverted; echo $?
7        # before
9        # after
```

## The MIR evidence

`kestrel dump mir -s verify` on the repro, before the fix:

```
; function: Test.main
bb0:
    %v0 = literal i64 200                          // @owned Int64
    %v1 = struct Test.Inverted { .0: %v0 }         // @owned Test.Inverted
    %v2 = uninit Test.Inverted                     // @owned Pointer[Test.Inverted]
    store_init %v2, %v1
    %v3 = begin_borrow_addr %v2, Test.Inverted     // @guaranteed Test.Inverted
    %v4 = copy_value %v3                           // @owned Test.Inverted
    end_borrow %v3
    branch %v4, bb1(%v2, %v4), bb2(%v2, %v4)
```

`branch %v4` — and `%v4 : @owned Test.Inverted` is the **whole struct**. The
`boolValue()` witness is never called. After the fix:

```
    %v5 = begin_borrow %v4                         // @guaranteed Test.Inverted
    %v6 = call @witness std.core.BooleanConditional.boolValue for Test.Inverted(@borrow %v5)
    end_borrow %v5
    branch %v6, bb1(%v2, %v4, %v6), bb2(%v2, %v4, %v6)
```

### Why nothing caught it

Both backends turn a `branch` into a truthiness test on whatever scalar the
condition value happens to lower to — LLVM does
`resolve_scalar(cond).into_int_value()` then `NE 0`, cranelift
`icmp_imm(NotEqual, cond, 0)`. Neither has any way to know the source type was a
protocol conformer, so neither can complain. The result is a live silent
miscompile in **both** directions:

| `Inverted.v` | raw-bit branch | `boolValue()` | agree? |
|---|---|---|---|
| `200` | then | `false` → else | no |
| `0`   | else | `true` → then  | no |

Every `BooleanConditional` conformer in the stdlib and in user code happened to
be one whose `boolValue()` agrees with its payload bits, so the bug never
surfaced. `std.core.Bool` in particular is a one-field wrapper over `lang.i1`
whose `boolValue()` is literally `{ self.value }`, so the whole language's
day-to-day `if` was correct by coincidence of layout.

An explicit `x.boolValue()` call in the condition lowered fine — the witness
dispatch machinery was never broken. Only the **implicit** path skipped it.

### Why the existing tests could not catch it

All 13 files under
`lib/kestrel-test-suite/testdata/builtins/boolean_conditional/` were
`diagnostics`-kind. They asserted the programs *type-check*, which they always
did. A diagnostics test is structurally incapable of noticing that a program
that compiles clean produces the wrong answer. That is exactly why this shipped.

(Two further traps in that directory: `boolean_conditional_with_and.ks` and
`boolean_conditional_with_or.ks` do NOT test what their names suggest. They call
`boolValue()` explicitly and combine the results with `lang.i1_and` /
`lang.i1_or`, so they exercise the intrinsic, not the `and` / `or` operators and
not the implicit condition path. They are left as-is.)

## The condition-position inventory

There are exactly **five** `emit_branch` call sites in mir-lower:

| site | position | status |
|---|---|---|
| `body/control.rs:65` (`lower_if`) | `if`, `else if`, desugared `while`, desugared `guard … else`, the non-binding link of `if let p = e, cond` and of multi-condition `while let p = e, cond` | **fixed** — one insertion covers all six |
| `body/pattern.rs:816` (`DecisionTree::Guard`) | `match` arm guard | **fixed** — the one condition position that is not a `HirExpr::If` |
| `body/pattern.rs:464` | the bool pattern split (`Constructor::True`/`False`) | out of scope, see below |
| `body/pattern.rs:543` | the string-literal match chain | not a user condition — branches on a `Matchable.matches` result, already `MirTy::Bool` |
| `body/mod.rs:1509` (`emit_guarded_destroy`) | synthesized drop-flag guard | not a user condition — branches on `self.bool_ty()`, already `MirTy::Bool` |

`HirExpr::If` being the single HIR shape behind six surface positions is what
made this a one-line fix rather than a six-site sweep.

### The bool pattern split is out of scope

`pattern.rs:419-436` branches on a `Constructor::True` / `Constructor::False`
decision-tree split without coercing. That is safe, and verified rather than
assumed: those two constructors are only ever produced from
`TypeShape::constructors` for `TypeShape::Bool`
(`kestrel-pattern-matching/src/constructor.rs:513`), and `TypeShape::Bool` has
exactly two sources (`constructor.rs:556`, `:576`) — the `lang.i1` intrinsic,
and a type conforming to `ExpressibleByBoolLiteral`. Merely conforming to
`BooleanConditional` never yields that decision-tree shape, so a conformer with
a non-identity `boolValue()` cannot reach this branch.

### What is NOT a condition position

- **`and` / `or` / `??`** — these are in `SHORT_CIRCUIT_OP_PROTOCOLS`
  (`kestrel-hir/src/body.rs:703`) and become `logicalAnd` / `logicalOr` /
  `coalesce` protocol calls. The branching happens *inside* `bool.ks`
  (`if self.value { other() } else { … }`), on a real `lang.i1` field — already
  the `MirTy::Bool` fast path.
- **`!` / `not`** — `UnaryOp::LogicalNot` is always a `logicalNot` ProtocolCall
  returning a value; it never produces a terminator.
- **`for … in`** — `desugar_for_loop` produces
  `loop { match $iter.next() { .Some(pat) => …, .None => break } }`. That is a
  `Match` on an Optional discriminant, not a condition.
- **single-`let` `while let`** — likewise `loop { match … }`, no condition.
- There is **no ternary operator** in the language (no `AstExpr::Ternary`), and
  **no assert intrinsic** that would branch on a user condition.
