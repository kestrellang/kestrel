// test: execution
// stdlib: true
// expect-exit: 0

// An `escaping` closure that reads a Copyable field (`r.id`) of a non-Copyable
// local captures a snapshot of that projection, so `r` itself is never moved
// and `eat(r)` afterwards is legal — the analyzer accepts it. MIR lowering
// disagrees and keeps a borrow of `r` live, so the program dies with an
// internal compiler error: "OSSA verify failed ... cannot take from
// ValueId(_): active borrow(s)". Found in the 2026-10 architecture review at
// 9767d2dc; the analyzer/MIR ownership "lockstep" contract
// (move_tracking.rs) has drifted.
// EXPECTED TO FAIL until mir-lower agrees that a projection-snapshot capture
// does not borrow its root.

module Test

struct Res: not Copyable {
    var id: Int64
}

func eat(consuming r: Res) -> Int64 { r.id }

@main
func main() -> lang.i64 {
    let r = Res(id: 8);
    let f: escaping () -> Int64 = { r.id };
    let eaten = eat(r);
    if eaten != 8 { return 1; }
    if f() != 8 { return 2; }
    0
}
