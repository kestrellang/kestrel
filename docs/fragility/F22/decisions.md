# F22 — decisions

See `problem.md` for the diagnosis. This file records the choices made while
fixing it, including the things deliberately *not* done.

## 1. One primitive, in `kestrel-hecs`, not four bespoke guards

**Decision:** add `kestrel_hecs::guard::OnDrop<F: FnMut()>` (new
`lib/kestrel-hecs/src/guard.rs`) and use it at all four push/pop sites.

A `PushGuard<'a>` holding `&'a RefCell<Vec<_>>` would work for
`QueryContext::active` and for nothing else: the other three stacks are
`thread_local!`s, and reaching one means calling `LocalKey::with`, so any guard
for them is already "a closure that runs on drop". Two abstractions for the
same job is exactly the duplication the project rule forbids, and the closure
form subsumes the reference form. `kestrel-hecs` is the right home — it is the
leaf crate both `kestrel-semantics` and `kestrel-mir-lower` already depend on,
and the primitive is about the query framework's own execution model.

`FnMut` rather than `FnOnce`: `Drop::drop` gets `&mut self`, so `FnOnce` would
need an `Option` dance for no gain.

## 2. `retain(|e| *e != key)` → `pop()` — verified, not assumed

The two semantics thread-locals removed their entry with `retain`, which is
`O(n)` and hides an assumption. `pop()` is equivalent here, and the reasoning
was checked before the simplification:

* A key is on the stack at most **once**. `execute` pushes `(entity, root)`,
  which is the query's key; the framework's own cycle detection panics on
  re-entering a query key that is already on the *active* stack, so a second
  `execute` for the same key cannot begin while the first is running. (This
  holds regardless of whether a given caller consults `computing_contains`
  first — the callers that skip that check, e.g. `solver.rs:2466`, are
  protected by the framework.)
* Pushes nest **strictly**: any nested `execute` completes entirely inside the
  outer one, and `OnDrop` guards unwind innermost-first, so the entry `pop`
  removes is always our own.

Where they differ, `pop` is the safer of the two: if the uniqueness invariant
were ever broken, `retain` would remove *both* copies while `pop` removes one.

## 3. The `verified_at` roll-back is unwind-only, and covers the re-execution

**Decision:** capture `old_verified_at` before the tentative mark and revert it
from an `OnDrop` gated on `std::thread::panicking()`.

The gate is required, not stylistic: on the normal path the mark is exactly
what `ensure_fresh` is supposed to leave behind. Unconditional revert would
disable verification-cycle breaking and re-verify everything.

The guard's **scope** is the whole `if let Some(memo_info)` arm, including the
`execute_query` fall-through — not just the `deps_unchanged` call. This was
found by test, not by inspection: the first attempt scoped the guard to
verification only, and
`panicking_verification_does_not_leave_a_query_marked_verified` still failed,
because the promise is equally broken when the *re-execution* panics — the memo
then keeps its old value behind the tentative mark. The mark is only made good
once `execute_query` writes a new `MemoEntry`; the guard has to live that long.
`ensure_fresh` now `return`s the re-execution explicitly so the guard stays
alive across it.

## 4. `clear_for_query` is replaced, not supplemented

**Decision:** delete `AccumulatorStore::clear_for_query` and add
`take_for_query(&mut self, &QueryKey) -> AccumulatorSnapshot` +
`restore_for_query(&mut self, &QueryKey, AccumulatorSnapshot)`.

`clear_for_query` had exactly one non-test caller (`execute_query`), and that
caller now needs the taken values in hand to survive an unwind. Keeping both
would leave a "clear" that silently means "lose these if you panic" sitting
next to the correct call — a trap for the next author, and two ways to do one
thing. Clearing is now expressible as dropping a snapshot, and
`take_for_query` is `#[must_use]` so dropping it is a deliberate act.

`restore_for_query` restores the **exact pre-take state**: types present in the
snapshot are overwritten with it, and types that appeared only during the
aborted run have that key cleared. A half-written diagnostic from a run that
ICEd is not a diagnostic.

Like the memo mark, restore runs only `if std::thread::panicking()` — a
successful execution legitimately replaces its old values.

## 5. `ctx.deps` guarded unconditionally, and it simplified the code

`deps` is the one site where the existing behaviour was already correct by
accident (see `problem.md`). The guard restores on **both** paths rather than
only on unwind, because the normal path was doing exactly the same restore by
hand two lines later — `self.deps.replace(saved_deps)` is deleted, not
duplicated. Nothing between `q.execute` returning and the end of
`execute_query` records a dependency, so moving the restore to scope-exit is
behaviour-preserving.

## 6. `mir-lower/ty.rs`'s opaque `panic!` → `ty_arena.error()`

Landed as a separate hunk, and justified independently of the guard: it reduces
the *supply* of panics, it does not make anything unwind-safe.

The precedent is in the same file. `resolve_callable_return_type`
(`ty.rs:63-72`) asks the identical question — "what concrete type did this
opaque's body produce?" — and already fails soft to `error()`. An opaque origin
with no concrete type is an upstream inference failure that has already
reported; aborting the whole lowering over it is strictly worse than lowering
the rest of the module.

## 7. Codegen: comments only

Both `define_all_functions` implementations catch panics per function and
trap-stub the failure. Neither touches any of these stacks, so there is nothing
to guard. Each got a comment pointing the next author at `OnDrop` rather than a
hand-written pop, because "this host catches panics" is invisible from inside
the code that would be tempted to add such state.

## 8. Tests: the framework's own recovery, not the compiler's

The two pre-existing tests in `lib/kestrel-hecs/tests/f22_unwind_repro.rs`
assert desired behaviour and were left byte-identical; they flip from red to
green. Three more in `f22_unwind_recovery.rs` cover the two defects found while
fixing (verified-mark roll-back, accumulator snapshot/restore, and that a
partial run's own output is discarded), and four unit tests in
`accumulator.rs` cover the take/restore round trip. All five integration tests
were confirmed failing against `HEAD` in a scratch worktree before the fix.

`World::query_exec_count()` is used as an **invariance** check — an RAII
refactor must not change how many times anything executes — not as an overhead
measure. The existing `sub_query_skips_when_unchanged`,
`backdating_skips_downstream_when_value_unchanged`, and `exec_count_tracking`
tests assert exact counts and were required to pass unedited.
