# kestrel-hecs

The entity component system and memoized queries everything else runs on.
The `hecs` skill says how to use it; `docs/architecture.md` and
`docs/snapshots.md` say how it works.

- **Only kestrel-ast-builder sets components.** Every later stage reads
  components and runs queries. Don't add a mutation downstream.
- **Query keys are small and `Hash + Eq`; outputs are `Clone + Hash`.** The
  output's fingerprint is what backdating compares, so put nothing in it that
  changes without the answer changing.
- **Recursion uses `CycleMode`**, not a side-channel stack. The default
  (`Panic`) means a cycle is a bug; pick `Fixpoint` or `Fallback` when the
  question is recursive by nature.
- **Report with `ctx.accumulate`.** Accumulated values are attributed to the
  query and replaced when it re-runs; returned diagnostics are not.
- **Push/pop side state goes through `guard::OnDrop`**, bound to a named local
  (`let _guard = ...`, never `let _ =`). Five hosts catch panics and keep going,
  and a pop skipped on unwind becomes a wrong answer for every later query on
  that thread.
- **`snapshot()` clones memos and accumulated values together.** A cache hit
  in the snapshot never re-files its values, so dropping one without the other
  loses diagnostics (audit F21).
- **Calling order is not load-bearing.** Invalidation reads each entity's
  `last_changed`, and the first mutation after a query starts a new revision.
  Don't add code that relies on `begin_revision()` being called at a
  particular moment.
