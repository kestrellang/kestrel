// test: execution
// stdlib: true
// expect-exit: 0

// docs/design/shared-box.md: `isUnique()` is `true` only when this handle is
// provably the sole owner — it backs copy-on-write forking. Sharing the box
// flips it to `false`; releasing the extra handle (here: the callee's local,
// dropped at return) restores `true`. Lifecycle is ordinary value semantics —
// share is `clone()`, release is the handle's `deinit`.
module Test

import std.memory.(RcBox)
import std.numeric.(Int64)
import std.core.(Bool)

// `inner` is a second handle scoped to this call; it is released on return.
func cloneAndReportShared(box: RcBox[Int64]) -> Bool {
    let inner = box.clone();
    box.isUnique()              // false while `inner` is alive
}

@main
func main() -> lang.i64 {
    let a = RcBox[Int64](5);
    if a.isUnique() == false { return 1; }        // fresh box: sole owner

    if cloneAndReportShared(a) { return 2; }      // shared while the clone lives

    if a.isUnique() == false { return 3; }        // the clone's release restored uniqueness
    0
}
