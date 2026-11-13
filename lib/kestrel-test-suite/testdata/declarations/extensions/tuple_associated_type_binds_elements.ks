// test: execution
// stdlib: true
// expect-exit: 0
//
// An associated type bound in `extend (X, Y): P { type Element = X }`
// projects through a generic `C.Element` with X/Y bound to the tuple's
// element types, and each protocol's `Element` is its own (the tuple
// conforms to two protocols that both name their associated type
// `Element`). Before the fix `C.Element` stayed the extension's unbound `X`
// (E109 "expected T0 got Int64").

module Test

protocol Container {
    type Element
    func first() -> Element
}

protocol Other {
    type Element
    func other() -> Element
}

extend (X, Y): Container {
    type Element = X
    func first() -> X { self.0 }
}

extend (X, Y): Other {
    type Element = Y
    func other() -> Y { self.1 }
}

func head[C](c: C) -> C.Element where C: Container { c.first() }
func tail[C](c: C) -> C.Element where C: Other { c.other() }

@main
func main() -> Int64 {
    let p: (Int64, Bool) = (7, true);
    let x: Int64 = head(p);
    if x != 7 { return 1 }
    let b: Bool = tail(p);
    if b == false { return 2 }
    0
}
