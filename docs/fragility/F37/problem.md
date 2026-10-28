# F37 — `find_inherited_assoc_type` recursed through protocol inheritance with no cycle guard

`medium` · `fragility` · crate: `kestrel-name-res`

## Symptom

A protocol inheritance cycle written with **qualified** paths, combined with any
associated-type reference that has to walk that cycle, aborted the compiler with
a hard stack overflow — not an ICE, not a diagnostic, a `SIGABRT`:

```kestrel
module Test

protocol A: Test.B {}
protocol B: Test.A {}

func take[T](x: T) -> T.Missing where T: A {
    x.read()
}
```

```
$ kestrel build e_qualified_cycle.ks

thread 'main' (2413085) has overflowed its stack
fatal runtime error: stack overflow, aborting
$ echo $?
134
```

The one-protocol degenerate form (`protocol Foo: Test.Foo {}`) aborted
identically.

This is worse than a normal compiler error in a test context: `libtest_mimic`
runs trials as threads in a single process and triage batches many tests per
process, so a stack overflow in one `.ks` file takes down the **whole batch**,
not just the offending trial.

## Root cause

`lib/kestrel-name-res/src/resolve_type.rs` `find_inherited_assoc_type` walked a
protocol's `Conformances` looking for an inherited associated type, resolving
each positive conformance to an entity and then **tail-recursing into it** — with
no record of which protocols it had already visited. Given `A: B` and `B: A`, it
recursed `A → B → A → B → …` until the stack ran out.

### E459 does not protect this

`circular_protocol_inheritance` (E459) *is* detected, and it fires correctly on
these programs. It does not help here: E459 lives in `ProtocolCycleAnalyzer`, a
`CompilationCheck` that **consumes** name-res queries. Resolution therefore runs
*underneath* the check that would have reported the cycle. Any "the analyzer
catches it" reasoning about name-res recursion is backwards.

## The duplicated walk

The decisive observation is that `find_inherited_assoc_type` was not merely
*similar* to `resolve_name.rs`'s `resolve_inherited_protocol_member` — it was the
**same walk, copied**. Both:

- take `(ctx, protocol, name, root, …)`,
- read `Conformances` off `protocol`,
- skip non-`Positive` items and non-`AstType::Named` conformance targets,
- resolve each target with `ResolveTypePath` at a *climbed* context,
- reject anything that isn't `NodeKind::Protocol`,
- call the **same** shared leaf `find_assoc_type` (`resolve_name.rs`, already
  `pub(crate)` and already imported into `resolve_type.rs`),
- and tail-recurse into the parent protocol.

They differed in exactly two places, and both differences were the `resolve_type`
copy being wrong:

| | `resolve_name.rs` (correct) | `resolve_type.rs` (deleted) |
|---|---|---|
| cycle guard | `visited: &mut HashSet<Entity>`, insert-or-bail at entry | none — unbounded recursion |
| resolution anchor | `ctx.parent_of(protocol)` — recomputed per level from the entity being recursed on | `ctx.parent_of(scope)`, where `scope` was threaded in from outside |

`resolve_name.rs` had even *documented* the guard and why it exists; the copy in
`resolve_type.rs` predated or ignored it.

## Second, independent bug: the drifting anchor

`find_inherited_assoc_type` computed `let resolve_ctx = ctx.parent_of(scope)` and
then recursed passing `resolve_ctx` as the next level's `scope`. So the
resolution context climbed **one additional ancestor per recursion level**:

```
level 0: parent_of(caller_scope)
level 1: parent_of(parent_of(caller_scope))
level 2: parent_of(parent_of(parent_of(caller_scope)))
```

A one-hop climb is intentional and correct — a conformance type path must resolve
relative to where it is *written*, i.e. the declaring protocol's own scope, and
the hop past the protocol itself avoids `ResolveName` rule 6 protocol-self
shadowing. But climbing *cumulatively* is not a policy, it is drift: a deep
inheritance chain reached from a nested caller walks clean out of its own module
and starts failing to resolve conformance targets that are plainly in scope.

## Why the bare-name cycle "worked"

`protocol A: B {}` / `protocol B: A {}` with a real assoc-type reference did
**not** crash before the fix. That was luck, not a guard: the drifting anchor
climbed past the module and off the top of the tree within a couple of levels, at
which point `ResolveTypePath` stopped finding `B`, the loop `continue`d, and the
recursion bottomed out. The bug's second half was accidentally masking its first
half. Writing the cycle with a qualified path (`Test.B`) makes the target
resolvable from *any* ancestor context — the drift no longer terminates the walk,
and the missing guard is exposed as a stack overflow.

That interaction is why the fix needs the regression test
`protocol_cycle_bare_two_way_assoc_ref_still_resolves.ks`: re-anchoring the climb
removes the accidental termination, so the real guard has to carry the weight,
and the bare-name case that *did* resolve must keep resolving.

## Reproductions

Under the F37 scratchpad (`…/scratchpad/f37/`):

| file | shape | before | after |
|---|---|---|---|
| `e_qualified_cycle.ks` | `A: Test.B` / `B: Test.A` + `T.Missing` | stack overflow, exit 134 | E459 + "cannot find type" |
| `g_qualified_selfcycle.ks` | `Foo: Test.Foo` + `T.Missing` | stack overflow, exit 134 | E459 + "cannot find type" |
| `b_two_cycle.ks` | bare `A: B` / `B: A` + existing `T.Item` | E459 only (assoc resolves) | E459 only (unchanged) |
| `d_where_assoc.ks` | `P: Q` / `Q: P` + nested `T.Iter.Nope` | E459 + not-found | E459 + not-found |

## Post-fix behavior

The walk returns `None`. E459 surfaces from the independent
`ProtocolCycleAnalyzer`, plus a truthful secondary "cannot find type" for the
unresolvable associated-type reference. Both are correct and both are kept — see
`decisions.md`.
