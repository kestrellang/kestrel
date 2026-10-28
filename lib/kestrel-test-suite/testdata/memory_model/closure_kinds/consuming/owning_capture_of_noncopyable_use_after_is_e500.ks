// test: diagnostics
// stdlib: true

// docs/design/closures.md, owning-capture table: a non-Copyable place owned by
// the frame is MOVED into a `consuming` closure's environment at creation, so
// "the source afterwards" is "dead — later use is E500". The body here only
// borrows; the capture itself is the move.
module Test

import std.numeric.Int64

struct Res: not Copyable {
    var id: Int64
    func peek() -> Int64 { self.id }
    deinit { }
}

func onDone(consuming f: consuming () -> ()) { f(); }

func main() -> lang.i64 {
    let r = Res(id: 1);
    onDone({ () in let _ = r.peek(); });
    let x = r.peek(); // ERROR(E500)
    let _ = x;
    0
}
