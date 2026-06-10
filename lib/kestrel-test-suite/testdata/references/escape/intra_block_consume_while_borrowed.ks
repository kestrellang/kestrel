// test: diagnostics
// stdlib: false

// tests.md `intra_block_consume_while_borrowed`: consuming the owner
// while a reference into it is live in the same block is rejected. The
// OSSA `try_consume` gate attributes the conflict to the live ref and
// reports E498 (an unattributable conflict stays an ICE — that's a
// lowering bug, not user error).
module Test

struct Res: not Copyable {
    var v: lang.i64
}

struct Box {
    var r: Res
    func peek() -> &Res { self.r }
}

func use() {
    let b = Box(r: Res(v: 1));
    // hold the ref open as a call argument while consuming the owner
    observe(b.peek(), eat(b)); // ERROR(E498)
}

func observe(r: Res, u: ()) {}
func eat(consuming b: Box) -> () {}
