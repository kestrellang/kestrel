# G8 / G9 / G10 — decisions

## 1. Two tiers: the *rule* in `kestrel-hir`, the *walk* in `kestrel-analyze`

The obvious move is to lift one `contains_break` into a shared helper and have
everyone call it. That fixes the four clones and leaves the deeper problem: MIR
lowering's `find_loop` would still be a second, independent statement of the
label rule, and `initializer.rs` — which cannot use a syntactic walk at all —
would be a third.

So the split is by *what is actually shared*:

**Tier 1 — the atomic fact, in `kestrel-hir`** (`body.rs`):

```rust
pub fn label_selects_loop(use_label: Option<&str>, loop_label: Option<&str>) -> bool {
    match use_label { None => true, Some(l) => loop_label == Some(l) }
}
```

This is Kestrel's whole label rule, applied by a caller to its own
innermost-first loop stack. All three label consumers now read it:
`mir-lower`'s `find_loop`, `analyze`'s `control_flow` walk, and
`initializer.rs`'s frame search. `kestrel-hir` is where it belongs because it is
a property of the HIR node pair (`Break.label`, `Loop.label`) and nothing else —
no walking, no context, no diagnostics.

**Tier 2 — the walk, in `kestrel-analyze`** (`body/control_flow.rs`):

```rust
pub(crate) fn block_contains_break_for(hir, block, target: Option<&str>) -> bool
```

### Why the walker does NOT go in `kestrel-hir`

`kestrel-hir` is a pure-data crate. It has arenas, node definitions, and lookup
tables; it contains **zero** traversal logic, and adding the first walker sets a
precedent that every consumer's traversal belongs there. The walk also is not
consumer-neutral: it needs the `Sugar`-transparency rule (a `kestrel-hir-lower`
convention, not a HIR invariant) and analyzer-specific stop rules (a closure is
a fresh break scope *for divergence purposes*; MIR lowering handles closures
by lowering a separate body and never asks). Those are analyze-layer decisions
about what a break "reaches", not facts about the data structure.

The rule, by contrast, is genuinely shared across crates — which is why the two
tiers land in two different places.

## 2. `find_loop` becomes a `.find()`, and the equivalence was checked

```rust
// before
match label {
    Some(label) => self.loop_stack.iter().rev().find(|l| l.label.as_deref() == Some(label)),
    None => self.loop_stack.last(),
}

// after
self.loop_stack.iter().rev().find(|l| kestrel_hir::label_selects_loop(label, l.label.as_deref()))
```

Behavior-identical, both arms:

- `label == None` → `label_selects_loop` returns `true` unconditionally, so
  `.rev().find(|_| true)` yields the first element of the reversed iterator,
  which is `loop_stack.last()`. Same for the empty stack: both yield `None`.
- `label == Some(x)` → the predicate reduces to
  `l.label.as_deref() == Some(x)`, the original closure verbatim.

Verified by running: the full `kestrel-mir-lower` test set and the `*loop*` /
`*break*` / `*guard*` triage subsets (617 tests) were unchanged across the
refactor, and Stage 1 as a whole was a suite no-op.

The point is not to shorten `find_loop`. It is that after this change, analyze
and MIR share the rule *by construction* — a future change to label semantics
has one edit site, and the two cannot silently disagree the way they did for
G9.

## 3. The `crossed` flag is load-bearing and is not expressible via `target`

`block_contains_break_for(hir, block, target)` alone cannot answer the
question, because the same `target` means two different things depending on
depth:

```
Break { label } => if crossed { target.is_some() && label.as_deref() == target }
                   else       { label_selects_loop(label.as_deref(), target) }
```

- **Not crossed** (still directly inside the loop `target` names): an unlabeled
  `break` exits it. `label_selects_loop(None, target)` is `true`.
- **Crossed** (inside a nested loop): an unlabeled `break` belongs to the
  *inner* loop and must not count. Only an explicit `break target` reaches back
  out.

Collapsing the two into one predicate would either credit every nested bare
`break` to the outer loop (the pre-existing bug in the other direction) or
refuse to credit an outer loop's own bare `break`. The distinction is
*positional*, and `target` carries no position. Hence a separate flag, seeded
`false` at the entry point.

Note the asymmetry this produces for an **unlabeled** loop: once crossed,
`target.is_some()` is false, so nothing below can exit it. That is correct —
an unlabeled loop has no name, so no `break` from inside a nested loop can
ever name it.

### The `Loop` shadow guard is the subtlest line

```rust
HirExpr::Loop { label: inner, body, .. } => {
    if target.is_some() && inner.is_some() && inner.as_deref() == target {
        return false;                       // shadowed
    }
    contains_break_in_block(hir, body, target, /*crossed=*/ true)
}
```

`outer: loop { outer: loop { break outer; } }` — the inner `break outer`
resolves to the **inner** loop, because `find_loop` searches innermost-first
and stops at the first match. So the outer loop has no exit and *is* infinite.
Getting this backwards would reintroduce G9 in mirror image: an outer loop
credited with an exit it does not have, suppressing a legitimate E001.

It has its own unit test (`inner_loop_reusing_the_label_shadows_it`), as do
the two-level G9 shape, the closure boundary, and the `Sugar` wrapper.

## 4. G10 keeps its stack — it is the honest exception

`initializer.rs` was the one site where "just call the shared predicate" is
wrong, and it is worth being explicit about why, because the shape looks like
the other four.

`loop_break_stack` does not ask "is there a break?". It captures an `InitState`
— the set of definitely-assigned fields — at **each reachable break**, and
merges them to produce the post-loop state. That is reachability-aware: a
`break` behind an already-diverged path contributes nothing, and two breaks
assigning different fields merge to their intersection. `block_contains_break_for`
is a syntactic existence check and cannot express any of it. Replacing the
stack with the predicate would lose the field-set merge entirely.

So G10 is fixed at the **pairing**, not the walk: frames become
`Vec<(Option<String>, Vec<InitState>)>`, and the `Break` arm swaps `.last_mut()`
for an innermost-first search under `label_selects_loop`. G9 and G10 end up
sharing the **predicate**, not the walker — which is the correct amount of
sharing, and is why Tier 1 exists as a separate thing from Tier 2.

This is also the general principle, now written into `AGENTS.md`: a shared
*fact* moves to `control_flow.rs`; a fact that needs analyzer-specific state
carried alongside the walk stays local. Without that second half,
`control_flow.rs` becomes a dumping ground for every loop traversal in the
crate.

## 5. The `AGENTS.md` contradiction had to be fixed, not just the code

`lib/kestrel-analyze/AGENTS.md` §5 read:

> Analysis-specific logic (e.g., **divergence checking, control flow
> analysis**) lives as private functions in the analyzer file. Only span
> extraction and entity info helpers go in `util.rs`.

That is not a description of a mistake — it is an instruction that names this
exact category and mandates the duplication. And it contradicts the same
file's "One analyzer per fact" section, which says two analyzers computing the
same thing must be merged because they drift. An agent following §5 wrote the
fourth `contains_break` copy; an agent following "One analyzer per fact" would
not have written the second.

Fixing four call sites and leaving §5 alone guarantees a fifth copy. So §5 now
distinguishes *logic* (private by default — unchanged) from a *fact with
multiple consumers* (belongs in `body/control_flow.rs`, `pub(crate)`, pure:
`&HirBody` in, `bool`/data out, no `TypedBody`, no `BodyContext`, no
diagnostics). It cites `block_contains_break_for` as the precedent and names
`initializer.rs`'s `InitState` stack as the counter-example that stays local.
`util.rs`'s scope is untouched — it remains span/entity helpers only, and the
new module is deliberately *not* part of it, because `util.rs` is a grab bag and
`control_flow.rs` has a type discipline.

## 6. Staging

Five commits, in this order, for these reasons.

**Stage 1 — the shared pieces, unused.** `label_selects_loop`,
`control_flow.rs` with its unit tests, the `find_loop` refactor, the
`AGENTS.md` amendment. Deliberately no behavior change, so the equivalence
argument in §2 is testable in isolation: if the suite moves at Stage 1, the
`find_loop` rewrite is wrong and nothing else can be blamed.

**Stage 2 — all four triads at once, atomically.** This one is not
negotiable. `dead_code` and `exhaustive_return` answer *the same question about
the same loop* and their answers are combined by the reader: on the G9 shape,
`exhaustive_return`'s missing E001 and `dead_code`'s false E002 currently
cancel out into "no diagnostics". Fix one and the file reports a diagnostic
that contradicts the other analyzer's model of it. A bisect landing between
them sees a state that never existed in review and is strictly more confusing
than today's uniform-wrong. The uniformity is the only good property the bug
has; do not break it halfway.

**Stage 3 — G8's `guard.rs` fix, with its `for`-in-guard-else pinning test.**
Separable from Stage 2 (different analyzer, different code) and needs its own
before/after, so it gets its own commit.

**Stage 4 — the deferred G11 `Sugar` arm on `guard.rs`'s `expr_diverges`.**
Must come *after* Stage 3, not with it. `guard.rs` currently cannot see through
a `Sugar` wrapper, so a `for` loop in a `guard ... else` is invisible to it
(falls to `_ => false`, "does not diverge") and E003 fires for the right answer
by the wrong reason. Adding the `Sugar` arm makes the `for`'s desugared
`Loop` visible — and if that arm shipped *before* Stage 3's break check, the
`Loop { .. } => true` arm would then accept `for x in xs { }` in a guard-else,
turning a correct rejection into a new false accept. Order matters.

Stage 3 therefore lands a pinning test — a `for` in a guard-else must be E003 —
*before* Stage 4 can change what `expr_diverges` sees. **If that test fails
after Stage 4, the arm is wrong, not the test.**

**Stage 5 — G10's frame keying.** Independent of everything above; last because
it is the smallest and touches an analyzer nothing else in the campaign
touches.
