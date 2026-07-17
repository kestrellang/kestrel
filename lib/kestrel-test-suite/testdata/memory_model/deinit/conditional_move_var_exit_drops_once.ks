// test: execution
// stdlib: true
// expect-exit: 0

// #107 (var flavor): a `var` moved out on only one arm of an `if` must drop
// exactly once — via the flag-guarded destroy at lexical scope exit — on
// both the moved and the kept path. Before the scope-exit guarded-destroy
// fix, terminating exits (including function fallthrough) DestroyAddr'd the
// MaybeUninit slot unconditionally: a double-free on the moved path.

module Test

import std.numeric.Int64

public var deinit_count: Int64 = 0;

struct Res: not Copyable {
    var id: Int64
    deinit {
        deinit_count = deinit_count + 1;
    }
}

func consume(consuming r: Res) {}

// Fallthrough function exit with a conditionally-moved var.
func fallthroughExit(cond: Bool) {
    var a = Res(id: 1);
    if cond {
        consume(a);
    } else {
    }
}

// Early-return exit with a conditionally-moved var.
func earlyReturnExit(cond: Bool) -> Int64 {
    var a = Res(id: 2);
    if cond {
        consume(a);
    } else {
    }
    if deinit_count > 100 { return 99; }
    return 7;
}

@main
func main() -> lang.i64 {
    fallthroughExit(true);
    if deinit_count != 1 { return 1; }
    fallthroughExit(false);
    if deinit_count != 2 { return 2; }

    deinit_count = 0;
    let _r1 = earlyReturnExit(true);
    if deinit_count != 1 { return 3; }
    let _r2 = earlyReturnExit(false);
    if deinit_count != 2 { return 4; }
    0
}
