// test: execution
// stdlib: true
// expect-exit: 0

// Pins the canonical `consuming` hand-off from docs/design/closures.md
// ("`consuming`: one-shot hand-off"): a non-Copyable resource moves INTO the
// owning environment at closure creation and back OUT of the body when the
// single call runs. The resource must be deinitialized exactly once.
module Test

import std.numeric.Int64

public var deinit_count: Int64 = 0;

struct Res: not Copyable {
    var id: Int64
    deinit { deinit_count = deinit_count + 1; }
}

func transfer(consuming r: Res) { }

func onDone(consuming f: consuming () -> ()) { f(); }

@main
func main() -> lang.i64 {
    let file = Res(id: 3);
    // `file` moves into the owning environment, then out again inside the body.
    onDone({ () in transfer(file) });
    if deinit_count != 1 { return 1; }
    0
}
