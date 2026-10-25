// test: diagnostics
// stdlib: false

// docs/design/closures.md — owning capture table: a non-Copyable place owned by
// the frame is MOVED into an `escaping` environment at creation. The source is
// dead afterwards, so a later use is an ordinary use-after-move (E500). View
// kinds would merely freeze `r`; only owning kinds move it.
module Test

@builtin(.Copyable)
protocol Copyable {}

struct Res: not Copyable {
    var id: lang.i64
    func value() -> lang.i64 { self.id }
}

func test() -> lang.i64 {
    let r = Res(id: 7);
    let g: escaping () -> lang.i64 = { r.value() };   // moves `r` into the environment
    r.id // ERROR(E500)
}
