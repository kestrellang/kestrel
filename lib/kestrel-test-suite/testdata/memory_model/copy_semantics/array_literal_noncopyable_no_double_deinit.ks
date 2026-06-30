// test: execution
// stdlib: true
// backends: cranelift,llvm
// expect-exit: 0

// Regression (#127): building an array literal with a non-Copyable element
// deinited each element TWICE — once during construction and once at scope exit
// (double-free; corrupted output for heap payloads). Root cause was NOT the
// array-literal lowering (its MIR is a clean move into the buffer) but
// `RcBox.init(consuming value: T)`: `value` is a bare type param (mono-dependent
// copy behavior), so the lowering emits `CopyValue` and defers cleanup to
// copy-prop. Block-local copy-prop couldn't collapse it because `value` is
// threaded across the post-`if let` allocation branch to the merge block where
// it's dropped — so at mono the surviving CopyValue expanded to a real
// `ArrayStorage.clone` (Cloneable) while the original `value` was still dropped.
// Fixed by `eliminate_cross_block_copies`: a copy whose operand is threaded by a
// jump into a single-predecessor block that only drops it becomes a move.

module Test

import std.numeric.Int64

public var drops: Int64 = 0;

struct Res: not Copyable {
    var tag: Int64
    deinit { drops = drops + 1; }
}

// Build the array in an inner scope: construction must not deinit, and the
// array's own scope exit (this function's return) drops each element once.
func buildAndCheck() -> lang.i64 {
    let rs = [Res(tag: 1), Res(tag: 2), Res(tag: 3)];
    if drops != 0 { return 1 };           // before fix: 3 (one per element, at construct)
    if rs.count != 3 { return 2 };
    if drops != 0 { return 3 };           // reading count does not drop
    return 0;
    // `rs` drops here → exactly 3 deinits total.
}

@main
func main() -> lang.i64 {
    let r = buildAndCheck();
    // After `rs` dropped: each element dropped EXACTLY once — no double-free
    // (was 6) and no leak (not 0). On any buildAndCheck failure this also
    // diverges from 3, so a non-zero exit results either way.
    if drops != 3 { return 5 };
    r
}
