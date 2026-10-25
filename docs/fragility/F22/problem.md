# F22 — push/pop bookkeeping that a panic skips, in a compiler with five panic-catching hosts

`medium` · `fragility` · crates: `kestrel-hecs`, `kestrel-semantics`,
`kestrel-mir-lower`

## Symptom

The compiler catches panics and keeps going. Five places do it:

| host | what it keeps across the caught panic |
| --- | --- |
| `CompilerDriver::infer_all` (`lib/kestrel-compiler-driver/src/lib.rs:69`) | **one** `QueryContext`, made before the loop; every *body* is wrapped individually at `:76` |
| the LSP compiler worker (`lib/kestrel-lsp/src/compiler_worker.rs:137`) | the thread and `state.compiler` — the whole session |
| the test harness (`lib/kestrel-test-suite/tests/file_tests.rs:160`) | libtest-mimic's fixed worker **pool**, so thread-locals survive from one `.ks` to the next; stdlib tests additionally snapshot the *same* cached world, so a leaked `Entity(n)` names the identical entity in the next test |
| cranelift `define_all_functions` | the module being emitted |
| LLVM `define_all_functions` | the module being emitted |

Four sites bracket work between a push and a hand-written pop, with no `impl
Drop`. When the bracketed work panics, the pop is skipped and the state leaks
into every later unit of work on that thread. The leak is not a lost cleanup;
it is a **wrong answer**:

* `QueryContext::active` (`query.rs:378`, field-scoped, so `infer_all`'s
  shared context is the exposed host) — a stale entry makes a later,
  non-recursive query panic with a fabricated `Query cycle detected`, and
  makes a top-level `ctx.accumulate` file its diagnostic under the dead key
  (`accumulate` uses `active.last()`).
* `OPAQUE_RESOLVE_STACK` (`mir-lower/src/ty.rs:275`, thread-local) — the
  origin looks like a cycle forever, so that opaque type lowers to
  `ty_arena.error()` for the rest of the process.
* `COMPUTING_COPY_SEMANTICS` (`semantics/src/lib.rs:489`, thread-local) — the
  nominal answers `Copyable` forever.
* `COMPUTING_STATICNESS` (`semantics/src/staticness.rs:199`, thread-local) —
  the nominal answers `Static` forever.

`lib/kestrel-hecs/tests/f22_unwind_repro.rs` reproduces the first two effects
against the exact shape of `infer_all` (one context, `catch_unwind` per unit of
work, keep going).

## Two further defects found while fixing it

Both are in `kestrel-hecs` and neither is a missing pop.

### 1. `ensure_fresh` leaves a memo marked verified that nothing verified

`query.rs:331-336` tentatively sets `verified_at = self.revision` *before*
calling `deps_unchanged`, to break verification cycles. The mark is a promise,
made good either by `deps_unchanged` agreeing or by `execute_query` rewriting
the memo. `deps_unchanged` recursively re-verifies sub-queries and can
re-execute them, so either half can panic — and then the memo keeps its **old
value** behind a `verified_at` that says "confirmed this revision". Every other
top-level query that reads it for the rest of that revision takes the stale
value silently.

It self-corrects at the next revision bump, so it is not "forever". The LSP is
the exposed surface: a `World` persists for a session and many requests share a
revision.

### 2. `clear_for_query` drops diagnostics that a panic then never re-produces

`execute_query` calls `accumulators.clear_for_query(qk)` *before* `q.execute`
(`query.rs:384`), because execute is about to re-push. If execute panics
partway, that query's previously-valid diagnostics are gone permanently — and
this is **independent of the `active` leak**: fixing the stack does not put
them back.

Note the audit's suggested fix ("clear after execute") is wrong: values are
pushed *during* `q.execute`, so clearing afterwards would delete exactly what
execute just produced.

## Why the cross-crate scope is one problem, not four

A stack-specific RAII type cannot reach into a `LocalKey` without re-entering
`.with()` anyway, so the useful shape at all four sites is identical: *a
closure that knows how to undo one push*. That is one ~10-line primitive,
`kestrel_hecs::guard::OnDrop`, and both `kestrel-semantics` and
`kestrel-mir-lower` already depend on `kestrel-hecs`.

## What is *not* affected

* `ctx.deps` (`query.rs:387-391`) is currently self-healing: every
  `execute_query` entry does `self.deps.take()`, zeroing the field regardless
  of what an unwind left, and nothing reads it in between. That self-healing is
  an unenforced accident of today's call topology, not a stated invariant, so
  it is guarded anyway as defence-in-depth.
* The two codegen hosts touch none of these stacks. No functional change was
  made there.
