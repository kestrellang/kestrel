# F37 — decisions

See `problem.md` for the diagnosis. This file records the choices made while
fixing it, including the things deliberately *not* done.

## 1. Delete the duplicate rather than add a second guard

**Decision:** delete `resolve_type.rs`'s `find_inherited_assoc_type` outright and
call `resolve_name.rs`'s `resolve_inherited_protocol_member` (widened from `fn`
to `pub(crate) fn`, body unchanged) from its one caller.

**Why not the obvious fix** — threading a `HashSet` through
`find_inherited_assoc_type` — is the whole point. The two functions were the same
walk over the same components against the same leaf (`find_assoc_type`), and one
of them already had the guard, the doc comment explaining the guard, and the
correct anchor. Adding a *second* guard would have left two copies that must be
kept in agreement forever, in a subtree where they had already silently diverged
once. Every future change to inherited-member lookup (new conformance item kinds,
visibility rules, protocol-extension targets) would need to land twice, and the
failure mode of forgetting is a stack overflow, not a test failure.

This is the project's single-source-of-truth rule applied to a walk instead of to
data. One walk, one guard, one anchor policy.

## 2. Re-anchor the climb to `parent_of(protocol)`

Adopting the shared function changes the resolution context for conformance
targets from `parent_of(externally-threaded scope)` to `parent_of(protocol)`,
recomputed fresh at each level. This is strictly more correct, not merely
different:

- A conformance type path is *written* in the declaring protocol's source. It
  must resolve relative to that protocol's own declaring scope. Anchoring it to
  whatever scope the original caller happened to be in is meaningless once the
  walk has moved to a different protocol — possibly in a different module.
- The old version compounded: each recursion level climbed one *further* ancestor
  (`parent_of(parent_of(…))`), so a long inheritance chain resolved its
  conformances from progressively more distant scopes and eventually from above
  the module root. That produced spurious "cannot find type" on deep chains
  reached from nested callers.

**The one-hop climb itself is kept and is deliberate.** Resolving a conformance
path from *inside* the protocol would hit `ResolveName` rule 6 protocol-self
shadowing and re-enter inherited-member search; hopping to the protocol's parent
scope is what makes the lookup well-founded. The hop is correct; making it
cumulative was the bug.

Because the accidental termination described in `problem.md` came *from* the
drift, removing it means the `visited` guard is now the only thing bounding the
walk. `protocol_cycle_bare_two_way_assoc_ref_still_resolves.ks` exists precisely
to pin that: a bare-name 2-cycle whose referenced associated type genuinely
exists must still resolve. It does (E459 only, no not-found).

## 3. Deliberately NOT fixed: `search_protocols_for_assoc`'s own climb — FOLLOW-UP

`resolve_type.rs` `search_protocols_for_assoc` still does:

```rust
let resolve_ctx = ctx.parent_of(scope).unwrap_or(scope);
```

on the *caller's* `scope`, to resolve a where-clause protocol bound. This is the
same-shaped anti-pattern as the one just removed — it resolves a protocol bound
relative to where the lookup started rather than where the bound is written.

**Not touched, on purpose:**

- It does not self-recurse, so it cannot be the unbounded-recursion crash. There
  is exactly one climb, not a compounding one.
- Fixing it changes which scope arbitrary where-clause bounds resolve in, across
  five call sites, and is a behavioral change with a much wider blast radius than
  a crash fix. It belongs in its own change with its own test sweep.

**Follow-up:** re-derive that anchor from the entity that *owns* the where clause
(the function/protocol/extension the constraint is written on) instead of the
threaded `scope`. Note that `search_protocols_for_assoc`'s signature keeps its
`scope` parameter solely for this line — if the follow-up lands, the parameter
likely disappears with it.

## 4. Deliberately NO cross-module cycle test

A cycle spanning two modules is a plausible-looking gap in coverage. Skipped
deliberately:

- The guard operates purely on resolved `Entity` identity and never consults a
  file or module. Nothing about it is file-sensitive, so a cross-module cycle
  exercises no new code path.
- The failure mode a cross-module test would nominally add — the anchor climbing
  out of the local module — is already forced by the same-file **qualified**
  2-cycle (`protocol A: Test.B`). A fully-qualified path stays resolvable from
  every ancestor context, which is exactly the climb-to-root condition, and that
  is the case that actually crashed.
- The multi-file harness is Rust-API-only; `triage` cannot run it. A test that
  the normal test loop never executes is worse than no test — it rots silently.

## Tests added

`lib/kestrel-test-suite/testdata/validation/cycles/`:

| file | covers |
|---|---|
| `protocol_cycle_two_way_qualified_assoc_ref_terminates.ks` | the crash: qualified `A: Test.B` / `B: Test.A` 2-cycle + assoc ref |
| `protocol_cycle_self_qualified_assoc_ref_terminates.ks` | the crash: qualified self-cycle `Foo: Test.Foo` + assoc ref |
| `protocol_cycle_bare_two_way_assoc_ref_still_resolves.ks` | **regression:** bare-name cycle whose assoc type exists must still resolve |
| `protocol_cycle_nested_where_bound_terminates.ks` | the other caller path — `resolve_assoc_type_nested` via `T.Iter: Q` |

All annotations were captured from actual post-fix compiler output, not guessed.
The secondary "cannot find type" is annotated as an *expected* error: the
associated type really does not exist, and special-casing "failed due to a cycle"
into the not-found message would be a lie about why resolution failed.

## Hazard note for future work here

Because a stack overflow aborts the entire `libtest_mimic` batch process rather
than a single trial, any new `.ks` testdata that probes unbounded recursion must
be added **together with** the fix and run under `--strategy isolated` first.
Landing such a file ahead of its fix takes down unrelated tests in the same
batch and reads as a mass failure.
