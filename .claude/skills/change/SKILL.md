---
name: change
description: Make a targeted behavioral change to an existing lib2 Kestrel compiler feature, keeping tests and docs in sync. Use when the user asks to change how an existing feature behaves — tweak a diagnostic, adjust inference for a case, modify lowering, fix a semantic bug — especially when the change spans one or two pipeline stages and doesn't need a full design doc. For brand-new features spanning many stages, use `feature` instead. For pure bug hunts with no known fix, use `debug-kestrel` / `debug-test`. For writing the test files, defer to `write-tests`.
---

# Making a Semantic Change in the lib2 Compiler

Scope: one behavior of one existing feature, touching one or two pipeline stages.
If the change needs a design doc, it belongs in the `feature` skill instead.

## Step 1 — Clarify the change

Get these explicit before touching code:

- **What behavior is changing?** Describe the old and new behavior in one sentence each.
- **Why?** Bug fix, design tweak, diagnostic improvement, new capability on an
  existing construct.
- **What's the new exact behavior?** If it's a diagnostic change, what's the
  new message text? If it's a lowering/inference change, what's the new
  observable result?

If any of these are ambiguous, ask before continuing. One clarifying question
now is cheaper than a redone PR.

## Step 2 — Explore impact

Use Explore (or `kestrel-pipeline`, which is faster for pipeline-routing
questions) to find:

1. **Implementation site** — which lib2 crate(s) own this behavior?
   - `kestrel-lexer` / `kestrel-parser` / `kestrel-syntax-tree` for surface syntax
   - `kestrel-ast-builder` for AST construction
   - `kestrel-name-res` for scope / visibility / import
   - `kestrel-hir-lower` for HIR lowering & desugaring
   - `kestrel-type-infer` for constraints and the solver
   - `kestrel-analyze` for validation diagnostics
   - `kestrel-mir-lower` / `kestrel-codegen-cranelift` for MIR and code generation
2. **Affected testdata** — grep `lib2/kestrel-test-suite/testdata/` for the
   feature's tests. Which will change expected output / `// ERROR:` text /
   `// expect-exit:` value? Which are still correct after the change?
3. **Documentation** — per-crate `docs/architecture.md` (or topic doc) if
   pipeline position / core types change. `docs/language/{feature}.md` for
   user-visible semantics. `.claude/skills/write-kestrel/SKILL.md` if a
   syntax/idiom/gotcha shifts.
4. **Memory** — check `/Users/dino/.claude/projects/-Users-dino-Documents-Projects-kestrel/memory/`
   for prior decisions about this feature. A lot of lib2 inference/MIR
   history is captured there.

Present findings before proposing a plan.

## Step 3 — Plan

Short checklist, not a design doc:

```markdown
## Change: <one-line summary>

### Implementation
- [ ] <file path>:<symbol> — <what changes>
- [ ] <file path>:<symbol> — <what changes>

### Testdata updates
- [ ] <testdata path>.ks — <why it needs updating> (new `// ERROR:` text / new
      `// expect-exit:` / removed because obsolete)
- [ ] <testdata path>.ks — …

### Testdata to add
- [ ] <new test path>.ks — <what it covers>

### Documentation
- [ ] <crate>/docs/<doc>.md — <what to update>
- [ ] docs/language/<feature>.md — <what to update>
- [ ] .claude/skills/write-kestrel/SKILL.md — <if syntax/idiom changed>
```

> **Confirmation gate.** User signs off on the plan before any code changes.
> Small wording tweaks ("use `N` field(s)" → "has `N` field(s)") still warrant
> a quick confirm — reviewer time is the expensive part.

## Step 4 — Implement

Make the minimum change that achieves the new behavior. Don't bundle drive-by
refactors — those go in a separate change.

While iterating, run triage with a **targeted pattern** (just the tests
touching this feature). Save the full suite for pre-commit.

```
# via the /triage skill — never `cargo test -p kestrel-test-suite2` directly
```

Debugging:
- Use `debug_trace!` plus `VERBOSE_DEBUG_OUTPUT=1`. No `eprintln!` / `println!`
  in the compiler source; the project rule in `CLAUDE.md` is firm on this.
- After 3 failed fix attempts in the same direction, stop and summarize
  what was tried / ruled out, then ask for guidance. Don't thrash; don't
  revert changes without consent.

## Step 5 — Update testdata

For each affected `.ks` under `lib2/kestrel-test-suite/testdata/`:

- **Old expectation is now wrong.** Update the expected behavior to the new
  one. For `diagnostics` kind, rewrite `// ERROR:` to the full new message
  (substring match is permissive but full messages are the project
  convention — see `lib2/kestrel-test-suite/AGENTS.md`). For `execution`
  kind, update `// expect-exit:`.
- **Test is obsolete.** Delete it; don't leave it ignored. Kestrel project
  rule is explicit: no `#[ignore]`.
- **New behavior needs coverage.** Add new `.ks` files. Delegate the authoring
  details to the `write-tests` skill — it has the format, header, and
  annotation rules.

**Firm rule (`CLAUDE.md`):** never modify a test just to make it pass. Only
update tests when the *old* expected behavior was actually wrong. If you find
yourself tempted to "fix" a test to silence a failure, stop and ask.

## Step 6 — Verify

- Targeted `/triage` run on the feature's tests — must be green.
- Full `/triage` run before commit. Read the results; don't report "done"
  off a started-and-ignored run.
- `cargo fmt` and `cargo clippy -p <changed crates>` clean.

## Step 7 — Update documentation

Only what actually changed — don't rewrite unrelated sections.

- **`lib2/kestrel-<crate>/docs/architecture.md`** — if pipeline position,
  core types, or module map shifted. See `lib2/AGENTS.md` for the required
  structure.
- **`docs/language/<feature>.md`** — if surface syntax or user-visible
  semantics changed (new / changed error messages count).
- **`.claude/skills/write-kestrel/SKILL.md`** — if a new gotcha or idiom is
  now worth warning future writers about.
- **Nearest `AGENTS.md`** — if the change establishes a new invariant,
  ordering constraint, or diagnostic-ID allocation that applies to the
  subtree. Per project rule, ask before adding an entry; don't silently
  edit.

## Step 8 — Summary

Short report to the user:

```markdown
# Change complete

## What changed
<old → new, one line>

## Files modified
- <path>: <what>

## Testdata updates
- <path>.ks: <old → new>

## Testdata added
- <path>.ks: <coverage>

## Documentation
- <path>: <what>
```

## Anti-patterns (don't do these)

- Modifying a test "to make it pass" when the old behavior was the correct
  spec. If the test was right, your change is wrong; if the test was wrong,
  say so explicitly in the plan.
- Bundling a diagnostic tweak with an unrelated refactor.
- Claiming a run is green without reading triage output.
- Running `cargo test -p kestrel-test-suite2` or `file_tests-*` directly —
  always go through `/triage` so history lands in `.triage/triage.db`.
- Silently adding a new pattern to an `AGENTS.md` without asking. Capture
  patterns as they come up, but ask first.

## When to use a different skill

- **`feature`** — new feature spanning many pipeline stages; needs design/plan
  gates.
- **`debug-kestrel` / `debug-test`** — the change is "figure out why X is
  broken" and there's no known fix yet.
- **`write-tests`** — authoring the `.ks` testdata (this skill decides *what*
  needs testdata; that one encodes *how*).
- **`kestrel-pipeline`** — just want "which file owns X?" without a workflow.
