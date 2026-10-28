// test: execution
// stdlib: true
// expect-exit: 0

// docs/design/closures.md — "Copy and Drop": reassigning an escaping-closure
// `var` releases its old handle before storing the new one. With one handle per
// environment, the reassignment must run the first environment's captured
// deinit immediately (drops == 1), and the second at scope exit (drops == 2).
module Test

import std.numeric.Int64

public var drops: Int64 = 0;
public var fail: Int64 = 0;

struct Res: not Copyable {
    var id: Int64
    func value() -> Int64 { self.id }
    deinit { drops = drops + 1; }
}

func makeThunk(id: Int64) -> escaping () -> Int64 {
    let r = Res(id: id);
    { r.value() }
}

func exercise() {
    var f = makeThunk(1);
    if f() != 1 { fail = 1; return; }
    if drops != 0 { fail = 2; return; }
    f = makeThunk(2);                       // releases the first environment here
    if drops != 1 { fail = 3; return; }
    if f() != 2 { fail = 4; return; }
}                                           // second environment released here

@main
func main() -> lang.i64 {
    exercise();
    if fail != 0 { return 1 }
    if drops != 2 { return 2 }
    0
}
