# G8 / G9 / G10 — every analyzer answered "can this loop exit?" for itself, and all of them were wrong

`high` · `false accept` + `false reject` + `silent miscompile` ·
crates: `kestrel-analyze` (`src/body/{dead_code,exhaustive_return,
definite_assignment,move_tracking,guard,initializer}.rs`),
`kestrel-mir-lower` (`src/body/control.rs`)

Six places in the compiler need the same fact — *does this loop have a `break`
that exits it?* — and six places computed it independently. Four were literal
copy-paste clones, one was a degenerate `=> true`, and one used a different
mechanism entirely. Every copy ignored `break`'s **label**, so `break outer`
was attributed to whichever loop happened to be innermost. The consequences run
from a spurious warning on checked-in testdata to a function declared
`-> Int64` that returns whatever is in the register.

## The true inventory

The audit recorded three copies for G9, four sites for G8, and five for G10.
The real counts are **four identical `contains_break` triads** and **six
divergence sites** that consume the fact:

| # | Site | Shape | Consumes the fact for |
|---|------|-------|----------------------|
| 1 | `dead_code.rs:309/321/329` | `block_/stmt_/expr_contains_break` triad | E002 unreachable-code |
| 2 | `exhaustive_return.rs:304/316/324` | identical triad | E001 missing-return |
| 3 | `definite_assignment.rs:496/508/516` | identical triad | E004 / divergence gate |
| 4 | `move_tracking.rs:2159/2171/2179` | identical triad (+ `loop_is_conditional`) | E500/E501 move checking |
| 5 | `guard.rs:167` | **no walk at all** — `HirExpr::Loop { .. } => true` | E003 guard-else divergence |
| 6 | `initializer.rs:603/618/637` | a *reachability* stack, not a syntactic walk | E005 uninitialized field |

Site 4 consumes it **twice** (the `diverged` gate at `:634` and
`loop_is_conditional` at `:2153`/`:2156`), which is where "six divergence
sites" comes from even though there are five files.

The four triads at #1–#4 were byte-identical apart from comment wording:

```rust
HirExpr::Break { .. } => true,                                  // label ignored
HirExpr::Loop { .. } | HirExpr::Closure { .. } => false,         // never descends
```

`HirExpr::Break { .. } => true` is wrong in one direction (a bare `break`
inside a nested loop is credited to the outer loop it does not exit) and the
`Loop { .. } => false` stop is wrong in the other (a `break outer` inside a
nested loop is *not* credited to the loop it does exit). In practice the second
dominates, because the outer `Loop` arm is only reached from the loop's own
body.

### Drift had already started

`dead_code.rs:345` alone carried a `Sugar` arm, added by the G11 fix last
cycle:

```rust
// Transparent wrapper — recurse into the desugared subtree.
HirExpr::Sugar { inner, .. } => block_contains_break(hir, body),
```

The other three copies did not. Benign at the time — no `for` loop in `while`
position had yet exposed it — but it is exactly the predicted failure mode:
one copy learns something, the rest keep the old answer, and the analyzers
disagree about the same file. `guard.rs`'s `expr_diverges` was *also* skipped
by that fix and still has no `Sugar` arm today.

---

## G9 — labeled breaks are attributed to the wrong loop

### Reproduction 1: false E002 on checked-in testdata

`lib/kestrel-test-suite/testdata/memory_model/deinit/break_from_nested_loop_3_levels.ks`
is an `execution` test that passes at exit 0 — so line 32 demonstrably runs:

```kestrel
outer: loop {
    let mid_r = Resource(id: 2);
    loop {
        let inner_r = Resource(id: 3);
        break outer;
    }
}

// inner_r and mid_r deinited by break; outer_r still alive
if deinit_count != 2 { return 1; }      // line 32
0
```

The dead-code walk stops at the nested `loop`, sees no break in the outer
body, concludes the outer loop is infinite, and warns:

```
line 32: warning: unreachable code [E002]
```

The harness only checks an `execution` test's exit code, so this had been
emitted on every build of the suite, invisibly, for as long as the file has
existed.

### Reproduction 2: E001 suppressed — and it links, and it runs

One level of nesting is the entire difference.

```kestrel
// g9a.ks — correctly rejected
func f() -> lang.i64 {
    outer: loop { break outer; }
    let z = 1;
}
```

```
error[E001]: function 'f' does not return a value on all code paths
  ┌─ g9a.ks:8:1
8 │ }
  │ ^ missing return
```

```kestrel
// g9b.ks — add one nested loop
func f() -> lang.i64 {
    outer: loop {
        loop { break outer; }
    }
    let z = 1;
}

@main
func main() -> lang.i64 { f() }
```

```
$ kestrel build g9b.ks
$ ./g9b; echo "exit=$?"
exit=0
```

No E001. No E002 either — the false E002 that *would* have flagged `let z = 1`
is suppressed by the same misattribution that suppressed E001, because
`exhaustive_return` and `dead_code` are wrong in the same way at the same
time. The function is declared `-> Int64`, has no `return` on any path, and
links and runs, returning whatever `f`'s return slot happened to hold.

MIR lowering, meanwhile, resolves `break outer` **correctly** — its `find_loop`
searches the loop stack by label. So MIR emits a real jump out of the outer
loop to a successor block the analyzers believe is unreachable. Analyze and
codegen disagree about the control-flow graph of the same function.

---

## G8 — the E003 gate has a hole, with a runtime consequence

`guard.rs:167` never had a walk at all:

```rust
// Infinite loop (no break) diverges
HirExpr::Loop { .. } => true,
```

The comment says "no break". The code does not check. Any loop in a
`guard ... else` block is accepted as divergent — which is exactly the
condition E003 exists to reject, since a non-diverging `else` falls through
past the guard with the guarded condition false.

The audit framed this as a `while`-desugaring problem. It is broader: the
`Loop` node is the same for `while true { break; }`, `for x in xs { break }`,
and a bare `loop { break; }`, so **any** breakable loop passes.

```kestrel
func f(x: lang.i64) -> lang.i64 {
    guard lang.i64_signed_gt(x, 0) else { while true { break; } }
    return 99;
}

func g(x: lang.i64) -> lang.i64 {
    guard lang.i64_signed_gt(x, 0) else { loop { break; } }   // also accepted
    return 99;
}

@main
func main() -> lang.i64 {
    let zero: lang.i64 = 0;
    f(zero)
}
```

```
$ kestrel build g8.ks
$ ./g8; echo "exit=$?"
exit=99
```

No diagnostic. `f` is called with `x == 0`, the guard fails, the `else` block
runs its loop, breaks out of it, falls straight through the guard, and returns
99 as though the guard had held. Every `guard` in the program is only as strong
as the fact that nobody has written a breakable loop in its `else`.

---

## G10 — E005 is skipped entirely

`initializer.rs` does not use a syntactic walk. It keeps a
`loop_break_stack: Vec<Vec<InitState>>` and captures the field-initialization
state at each **reachable** `break` — strictly stronger than any syntactic
predicate, because it knows which breaks are live.

The stack is pushed and popped correctly. The **pairing** is not:

```rust
// :637
HirExpr::Break { .. } => {
    if let Some(top) = vctx.loop_break_stack.last_mut() {
        top.push(state.clone());
    }
},
```

`.last_mut()` is "the innermost frame", unconditionally. A `break outer` from a
nested loop lands in the *inner* frame. The inner loop then pops a non-empty
frame and exits normally; the outer loop pops an **empty** one, concludes it is
an infinite loop, and at `:618-620` sets `state.diverged = true`. That flag
reaches the single gate at `:209`:

```rust
if !final_state.diverged {
    // ... the whole all-fields-initialized check
}
```

and the entire field check is skipped.

```kestrel
struct S {
    var a: lang.i64
    init() {
        outer: loop {
            loop { break outer; }
        }
    }
}

@main
func main() -> lang.i64 { let s = S(); 0 }
```

```
$ kestrel build g10.ks
$
```

Nothing. `S()` constructs successfully with `a` never stored. The
initializer's own analysis is fine — it is the label pairing that is broken,
and it fails *open*.

---

## Why one campaign

All three are the same fact answered by different code. G9 and G10 are the same
**rule** (`break label` selects a loop by searching the enclosing stack
innermost-first) implemented twice and wrong both times, while MIR lowering has
the correct implementation in `control.rs`'s `find_loop` and shares it with
nobody. G8 is the fact not being computed at all in a fifth place.

The `AGENTS.md` for `kestrel-analyze` sanctions the duplication. §5 read:

> Analysis-specific logic (e.g., divergence checking, control flow analysis)
> lives as private functions in the analyzer file.

which directly contradicts the same file's "One analyzer per fact" section:

> If two analyzers ask the same question, merge them. Two analyzers computing
> the same thing drift — one gets updated, the other doesn't, diagnostics
> disagree at the edges.

Fixing the four copies without amending §5 leaves the guidance that produced
them in place. See `decisions.md`.
