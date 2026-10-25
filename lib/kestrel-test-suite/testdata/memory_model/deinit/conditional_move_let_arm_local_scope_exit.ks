// test: execution
// stdlib: true
// expect-exit: 0

// #107: an arm-local `let` conditionally moved by a nested `if` must drop
// exactly once by the time its arm exits. Before let-via-address, the SSA
// merge-mask machinery mistimed the drop; after it, the fallthrough arm-exit
// walker leaked MaybeUninit slots until the flag-guarded destroy landed.

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

func armLocal(inner: Bool) {
    if true {
        let r = Res(id: 1);
        if inner {
            consume(r);
        } else {
        }
    } else {
    }
}

@main
func main() -> lang.i64 {
    armLocal(true);
    if deinit_count != 1 { return 1; }
    armLocal(false);
    if deinit_count != 2 { return 2; }
    0
}
