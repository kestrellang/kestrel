// test: diagnostics
// stdlib: true
//
// Regression (#162): moving a non-Copyable local into a struct memberwise
// initializer and then using it again must report use-after-move. Before the
// fix the move checker only tracked consuming calls / let-rebinds, so the
// invalid reuse sailed through to MIR and the OSSA verifier panicked.

module Test

import std.numeric.Int64

struct Res: not Copyable {
    var id: Int64
    deinit {}
}

struct Wrap: not Copyable {
    var inner: Res
}

@main
func main() -> lang.i64 {
    let r = Res(id: 1);
    let w = Wrap(inner: r); // r moved into the struct here
    let x = r.id; // ERROR: use of moved value 'r'
    return x.raw;
}
