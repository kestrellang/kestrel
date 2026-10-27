// test: diagnostics
// stdlib: false

// A non-Copyable projected Read capture is moved into an owning environment.
// It is a partial move (docs/design/partial-moves.md): only `h.resource`
// moves, so a later use of it is an ordinary E500 naming that field.
module Test

@builtin(.Copyable)
protocol Copyable {}

struct Res: not Copyable {
    var id: lang.i64
    func value() -> lang.i64 { self.id }
}

struct Holder: not Copyable {
    var resource: Res
}

func test() -> lang.i64 {
    let h = Holder(resource: Res(id: 7));
    let g: escaping () -> lang.i64 = { h.resource.value() };
    let _ = g;
    h.resource.value() // ERROR: use of moved value 'h.resource'
}
