// test: diagnostics
// stdlib: true
//
// Regression (#162): moving a non-Copyable local into an enum-case payload and
// then using it again must report use-after-move (was an OSSA verify ICE).

module Test

import std.numeric.Int64

struct Res: not Copyable {
    var id: Int64
    deinit {}
}

enum Holder: not Copyable {
    case Full(Res)
    case Empty
}

@main
func main() -> lang.i64 {
    let r = Res(id: 1);
    let h = Holder.Full(r); // r moved into the enum payload here
    let x = r.id; // ERROR: use of moved value 'r'
    return x.raw;
}
