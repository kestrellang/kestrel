# Verifying claims

This audit runs on empirical claims — "this program miscompiles", "this arm
discards the base", "the suite is green under X". Claims rot. Three ways:

1. **Measured on the wrong tree.** An agent worktree is created from `main`,
   not the working branch. Finding `G24`; it produced a false `high` finding
   (`G22`) and contaminated a second (`G23`) on its first outing.
2. **The code moved.** Line numbers drift; a cited function gets renamed,
   split, or fixed. A claim anchored to `resolve.rs:512` says nothing once the
   file changes.
3. **It was wrong when written and nobody re-ran it.** Three of the original
   audit claims and several first-pass analysis claims have been refuted.

The rule that follows from all three: **a claim without a reproduction and a
provenance stamp is a hypothesis.** Label it as one.

## The one thing not to build

Do **not** build a second verification harness alongside `kestrel-test-suite`.
A parallel repro-runner with its own manifest, its own expected-output format,
and its own notion of pass/fail is precisely the single-source-of-truth defect
this audit exists to find. It would need its own maintenance, drift from the
suite, and disagree with it eventually.

Behavioural claims belong in the suite. That is what the suite is.

## Tier 1 — behavioural claims become tests

> *"This program compiles clean and prints garbage."*
> *"Adding this clause silences a correct `E100`."*

These go into `lib/kestrel-test-suite/testdata/`, not into `temp/` or a
scratchpad. From the root `CLAUDE.md`:

> Tests should document the state of the compiler, they don't need to all pass.
> If a behavior is not working yet, you should add a test to ensure it gets
> fixed.

So a repro for an **unfixed** bug is a legitimate, expected-to-fail test. Write
the annotation for the behaviour the compiler *should* have. It fails today;
that failure is the record of the bug, and it flips to passing on the day
someone fixes it — automatically, with no doc to update.

Consequences worth stating plainly:

- Other agents running `triage` will see it fail. That is the point. Name the
  file so the reason is obvious (`assoc_projection_bound_cross_receiver.ks`,
  not `leak5.ks`) and reference the audit ID in a comment.
- An A/B pair beats a single repro. The bug is usually *"X behaves differently
  from X-minus-one-clause"*, and only the pair pins that.
- **Coverage gaps are claims too.** If every existing test of a feature has one
  type parameter, a bug that needs two is untestable by construction — say so
  in the finding, and add the two-parameter case.

Known limits of the suite, so you don't file a test that cannot see its own bug:

- The diagnostics matcher filters on `d.file_id == test_file_id`
  (`diagnostic_matcher.rs:191`), so a diagnostic anchored in `lang/std` is
  invisible to it. Finding `G23`.
- Nothing in CI builds a *debug* compiler, so `debug_assert!`-only failures
  ship green. Finding `G19`.

## Tier 2 — structural claims get anchors and stamps

> *"`conforms_to`'s `AssocProjection` arm discards the base."*

These cannot be tests; they are statements about source. Two rules:

**Anchor by symbol, not by line.** Write ``` `conforms_to`'s `AssocProjection`
arm (`resolve.rs:589`) ``` — symbol first, line as a convenience. A reader who
finds the line moved can still locate the code. A reader given only `:589`
cannot tell "moved" from "gone".

**Stamp the provenance.** Every empirical claim carries the commit it was
verified at:

```
[verified @ 296e3076, 2026-08-20]
```

No stamp means nobody has checked it on this branch. That is not a failure —
it is honest, and it is the difference between a lead and a fact. When a doc
mixes both, split it explicitly; `docs/fragility/G14-G17/problem.md` does this
with a VERIFIED / MEASURED-ELSEWHERE header.

## Tier 3 — the staleness preflight

Before producing **any** measurement — a build result, a test count, a sweep, a
line number — establish what you are measuring:

```bash
pwd                     # must be the parent checkout
git log --oneline -1
git rev-parse HEAD
```

and report it with the result. If `pwd` is under `.claude/worktrees/`, you are
probably on `main` and **173 commits behind** — stop and move.

Then build fresh, because a stale `target/` is the same bug one layer down:

```bash
cargo build --release --bin kestrel    # the binary is owned by the ROOT crate
```

A measurement reported without its base commit is unusable by the next reader,
who has no way to tell whether it still applies.

### Delegating a measurement

When you hand this to a subagent, the preflight goes in the prompt, not in your
hopes. Two failure modes to close explicitly:

- **Worktree isolation.** `isolation: "worktree"` bases the worktree on `main`.
  For anything measuring current behaviour, do not use it — say "work only in
  the parent checkout, do not create a worktree".
- **Unreproduced relay.** If you pass an agent's measurement upward without
  re-running it, you own it. Either reproduce the load-bearing ones yourself or
  mark them as the agent's, unverified.
- **`triage` is not the whole test suite.** It runs the `.ks` corpus only.
  Rust unit and integration tests inside a crate run only through
  `cargo test -p <crate>`. A claim like "the unit tests pass" needs that
  command, run on the current tree. G25 step 4 reported its entailment tests
  passing when the test target did not even compile (a `u32` passed where
  `usize` was expected). Nothing caught it for three commits, because every
  check in between was a `triage` run.

## When a claim is refuted

Refutation is cheap before implementation and expensive after. Record it, don't
delete it — a claim that was wrong once will be proposed again.

- Correct the source document in place, and note the refutation with its
  evidence where decisions are tracked (`docs/fragility/<ID>/decisions.md` has
  a *Refutations landed* section for this).
- If a finding is withdrawn entirely, keep its ID and mark it withdrawn with
  the reason. Do not recycle IDs; other documents reference them.
- Say what *caused* the bad claim, not just that it was wrong. `G22` was
  withdrawn as "does not exist on this branch"; the useful output was `G24`,
  the reason it looked like it did.
