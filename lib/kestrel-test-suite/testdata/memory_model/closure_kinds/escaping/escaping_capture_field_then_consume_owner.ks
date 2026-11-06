// test: execution
// stdlib: true
// expect-exit: 0

// An `escaping` closure that reads a Copyable field (`r.id`) of a non-Copyable
// local captures a snapshot of that projection, so `r` itself is never moved
// and `eat(r)` afterwards is legal. MIR lowering must copy the field without
// leaving `r` borrowed (it once did, an OSSA "cannot take" ICE).

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
