// test: diagnostics
// stdlib: true
//
// Regression (#162): moving a non-Copyable local into an aggregate and then
// using it again must report use-after-move (was an OSSA verify ICE). This
// used an array literal; arrays now require Copyable elements, so it moves
// into a struct instead.

module Test

import std.numeric.Int64

struct Res: not Copyable {
    var id: Int64
    deinit {}
}

struct Holder: not Copyable {
    var res: Res
}

@main
func main() -> lang.i64 {
    let r = Res(id: 1);
    let a = Holder(res: r); // r moved into the struct here
    let x = r.id; // ERROR: use of moved value 'r'
    return x.raw;
}
