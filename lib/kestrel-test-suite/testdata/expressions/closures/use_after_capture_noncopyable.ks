// test: diagnostics
// stdlib: true

module Test

import std.numeric.(Int64)

struct Res: not Copyable {
    var id: Int64
    deinit { }
}

func consume(consuming r: Res) -> Int64 { r.id }

// Regression (#177): capturing a non-Copyable value BY VALUE moves it into the
// closure environment. Using the root afterwards must be a clean use-after-move
// (E500), not an OSSA "consumed more than once" ICE at MIR. The move checker
// previously analyzed closure bodies in isolation and never leaked the capture
// move to the enclosing scope.
func main() -> lang.i64 {
    let r = Res(id: 7);
    let f = { () in consume(r) };   // r moved into f's environment
    let x = consume(r);             // ERROR: use of moved value
    let _ = f;
    let _ = x;
    0
}
