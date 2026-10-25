// test: execution
// stdlib: true
// expect-exit: 0

// Moving a non-Copyable field out of a locally-owned `let` in return
// position. With lets lowered via address (#107), the owned-field move-out
// path takes the whole struct out of its slot (marking it moved) and
// destructures it — the sibling-fieldless wrapper must NOT deinit the moved
// field a second time, and the vacated slot must not be destroyed at scope
// exit. Mirrors the `consuming self` shape of #145/#152.

module Test

import std.numeric.Int64

public var deinit_count: Int64 = 0;

struct Res: not Copyable {
    var id: Int64
    deinit {
        deinit_count = deinit_count + 1;
    }
}

struct Wrap: not Copyable {
    var inner: Res
}

func consume(consuming r: Res) {}

func moveFieldOut() -> Res {
    let w = Wrap(inner: Res(id: 3));
    w.inner
}

@main
func main() -> lang.i64 {
    let got = moveFieldOut();
    // Nothing dropped yet: the field was moved, not copied+dropped.
    if deinit_count != 0 { return 1; }
    consume(got);
    if deinit_count != 1 { return 2; }
    0
}
