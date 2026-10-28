# F43 — decisions

F43 is the determinism batch: three unrelated places where an unordered
container (or an unordered *counter*) reached compiler output. They were fixed
together because the reasoning is identical in each — "is this iterated into
something a user or a later pass can see?" — and the answers differed enough
that they are worth recording separately.

Landed in this batch: **F43b** (`PARAM_COUNTER`), **F43c** (HashSet reaching
diagnostic text), **F43d** (`module.witnesses` tail order). `F43a`'s
`extension_conflict.rs` claim was investigated and **refuted** — see below.

---

## 1. F43b — the synthetic-parameter counter was scoped, not seeded

`lib/kestrel-ast-builder/src/builders/params.rs` generated names for
destructured parameters from a process-lifetime `static AtomicU32`. Its doc
comment was false twice over: it claimed the format was `_0`/`_1` (it is
`_param_N`) and that the counter was "reset per parameter list via
`extract_params`" — nothing ever reset it. `grep -rn PARAM_COUNTER lib/`
returned the declaration and the single `fetch_add`, and nothing else.

The name is user-visible. E613 prints it verbatim:

```
error[E613]: required parameter '_param_0' cannot follow parameter 'a' which has a default value
```

**Decision: a local threaded through the helper, not a seeded or reset static.**

The alternative — keep the static and reset it at the top of `extract_params` —
is what the comment already *claimed* was happening, and it is wrong for the
same reason the original was: the test harness runs every test in one process
on parallel threads, so two threads building two unrelated files interleave
their resets. A local has no such window.

Scope is **one parameter list of one declaration**. Verified rather than
assumed: `extract_params` is called exactly three times in the crate
(`builders/function.rs:52`, `builders/function.rs:110`, `builders/subscript.rs`),
once per declaration, never re-entrantly. Default-value expressions do not
recurse back through it — they go through `lower::lower_default_value`, a
separate path.

Precedent followed: `builders/function.rs`'s `let mut opaque_index = 0u32;`.

The blast radius was worst in the LSP, which holds one long-lived `Compiler`
(`compiler_worker.rs`): the same unedited source produced `_param_0`, then
`_param_7`, then `_param_23` across rebuilds.

Recorded as a rule in `lib/kestrel-ast-builder/AGENTS.md`, adjacent to the
existing "component payloads must have deterministic order" rule. It is the same
class of defect with a different source of nondeterminism — process history
rather than hash order.

## 2. F43c — `IndexSet`/`IndexMap`, deliberately not `BTreeSet`/`BTreeMap`

Two sites, both feeding hash-container iteration into diagnostic output.

### `body/initializer.rs` — message *text*

`all_fields` is `join(", ")`ed into E005's message and into the E008 and E009
notes. Measured before the fix: **25 builds of one four-field repro produced 17
distinct orderings** (an earlier measurement on a different repro gave 23/25).
After: 25/25 identical.

**Decision: `IndexSet`, not `BTreeSet`.** Source-declaration order is free —
`util::children_of_kind` already yields it, so an `IndexSet` preserves it at no
cost — and "the order you wrote the fields in" is better UX than alphabetical.
A `BTreeSet` would have been equally deterministic and strictly worse to read.

**`let_fields`, `InitState::assigned` and `InitState::let_assigned` stay
`HashSet`.** Verified membership-only: they are `contains`/`insert` targets and
are never iterated for display. `InitState::merge` iterates them, but only into
another set — the result is order-insensitive. Converting them would be churn
with no observable effect, and would obscure which set is the load-bearing one.

### `decl/duplicate_callable.rs` — emission *order*, not message text

Filed as the same defect; it is not. `seen: HashMap<DuplicateKey, Vec<..>>` is
iterated to *emit* diagnostics, so the hash order was the order E426s appeared
in the output — the individual messages were always correct. Measured: **20 runs
→ 14 distinct orderings.**

Same one-word fix (`IndexMap`), and because the map is populated from
`children_of`, groups now report in source order.

### The `extension_conflict.rs` claim is REFUTED

The audit listed `extension_conflict.rs` alongside the two above. Neither
candidate file has the defect:

* `lib/kestrel-analyze/src/decl/extension_conflict.rs` contains no `HashSet` and
  no `format!` at all.
* `lib/kestrel-analyze/src/compilation/extension_conflict.rs` builds its
  messages by interpolating scalars drawn from `Vec`-ordered sources.

Neither was touched.

## 3. F43d — `module.witnesses` order is load-bearing, and no one owned it

`passes/clone_shim.rs` built `shim_map: HashMap<Entity, Entity>` and iterated it
straight into `module.add_witness`, which is a plain `Vec::push`.

**Currently unobservable**, and the evidence says so: 12 MIR dumps at 3 stages
were byte-identical. The reason is one invariant deep — every shim witness's
`implementing_type` is a distinct nominal, so no two shim witnesses can ever tie
in `select_most_specific`, so their relative order never decides anything.
Nothing states or enforces that invariant. It is the kind of thing that holds
until someone adds a second witness kind to the same loop.

Order *is* load-bearing at `mono/witness.rs:304` (`select_most_specific`, greedy
from candidate 0), and read positionally at `:272` and `:603`.

**Decision: `IndexMap` (already a `kestrel-mir` dependency).** No blast radius:
every positional use of `witnesses` is an index into the same slice within one
in-process run; `WitnessDef` derives only `Debug, Clone, PartialEq`; and neither
`kestrel-mir` nor `kestrel-compiler` depends on serde or bincode, so no witness
index is ever serialized across a process boundary.

**Also changed: the drain is now front-to-back instead of `worklist.pop()`.**
Nothing pushes to `worklist` inside the loop, so `pop()` was a plain reverse
iteration — deterministic, but it made the witness tail and the shim entity
numbering run backwards relative to `module.structs`. Since the unit test asserts
a *specific* order (see below), the order it asserts should be the sensible one.

`clone_impls`, `has_user_clone` and `closure_env_entities` were checked and left
alone: `has_user_clone` and `closure_env_entities` are membership-only, and
`clone_impls` is iterated only to write one independent field per key, so its
order cannot be observed. Its `.chain()` precedence (user clone overwrites shim)
is chain order, not hash order, and is unaffected.

### `select_most_specific`'s doc comment was wrong and is now corrected

It claimed that for genuinely incomparable overlaps "a deterministic candidate
is chosen". That is true only by accident. The scan keeps `best` unless a later
candidate is *strictly* more specific, so a tied pair resolves to whichever
appears **first** in `candidates` — i.e. first in `witnesses`. The function
cannot make itself deterministic and does not try to; it is deterministic only
because every producer of `module.witnesses` happens to be deterministically
ordered.

The corrected comment says that explicitly and names the `clone_shim.rs`
`IndexMap` as the dependency. Two new tests in `mono/witness.rs` pin it: one
that a unique global minimum is found from any position, and one that a
genuinely incomparable pair (`Pair[i64, T]` vs `Pair[U, str]`) resolves to the
first candidate.

## 4. Test placement — where a fixture *cannot* work

Two of the four behaviors here are invisible to the `.ks` test harness, and the
tests were placed accordingly rather than written as fixtures that would pass
vacuously.

**E008/E009 notes → `kestrel-analyze` unit tests.** The harness's
`diagnostic_matcher` does not compare `notes` at all: `matches_annotation` pairs
an annotation to a diagnostic by `(line, severity, message-substring)` and never
looks at the note list. `grep -rn note lib/kestrel-test-suite/src/*.rs` returns
nothing. A fixture annotating the note text would have passed no matter what the
note said.

**`duplicate_callable` emission order → a loop-N `kestrel-analyze` unit test.**
Same reason from the other direction: the matcher pairs annotations by
`(line, message-substring)`, not by position, so it is structurally incapable of
observing the order diagnostics come out in. The test calls the analyzer 50
times in-process over a source with four distinct duplicate-key groups and
asserts identical output order every time.

**F43d → assert the specific order, not "the same twice".** A "same every run"
check passes on any deterministic order, including a wrong one. The test asserts
the witness tail equals `module.structs` iteration order. A second test keeps
the reproducibility check as a cheaper complement.

**Non-alphabetical names everywhere.** The E005 fixture and the analyze unit
tests use `width, height, alpha, depth`; the duplicate-callable test uses
`alpha, beta, gamma, delta` in that declaration order. Declaration order is then
distinguishable from a `BTreeSet`/`BTreeMap`'s alphabetical order as well as
from any hash order — a fixture whose fields happen to be alphabetical would
pass under a `BTreeSet` and silently stop testing the thing it names.

All five new `kestrel-analyze` tests and both new `clone_shim` tests were
verified non-vacuous by reverting the container change and confirming they fail.

---

## Follow-up (deliberately NOT done here): sort diagnostics at the aggregation boundary

F43c is a symptom. The systemic version is that **nothing sorts diagnostics
anywhere in the compiler.**

`lib/kestrel-analyze/src/lib.rs`'s three aggregators — `analyze_bodies`
(`:246-270`), `analyze_decls` (`:278-302`), `analyze_compilation` (`:310-333`) —
each just `.extend()` in query-call order. The driver adds no sort of its own.
So the final diagnostic order is (entity iteration order) × (analyzer
registration order) × (whatever order each analyzer happened to build its own
`Vec` in).

The consequence: **every analyzer that builds a `Vec<AnalyzeDiagnostic>` from a
hash container has the `duplicate_callable` bug**, whether or not it has the
`initializer` bug. Fixing them one at a time is unbounded work and each fix can
silently regress.

A single stable sort-by-span at the aggregation boundary would be the real
single-source-of-truth fix, and would make each analyzer's internal container
choice stop mattering for *emission order* (it would still matter for *message
text*, which is why F43c's `IndexSet` is not redundant with it).

It is not done here because it changes diagnostic ordering compiler-wide: every
`.ks` fixture with multiple annotations, every LSP diagnostic list, and every
snapshot of compiler output is potentially affected. It needs its own audit and
its own full-suite run, not a ride-along in a determinism batch.
