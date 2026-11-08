// test: execution
// stdlib: true
// expect-exit: 0
//
// Methods from `extend (X, Y)` are callable directly on a tuple value, with
// X and Y bound to the tuple's element types — the same way `Box[T]`'s
// extension methods see `T` at the receiver's argument.

module Test

protocol Weighed {
    func weight() -> Int64
}

struct A: Weighed {
    var v: Int64
    func weight() -> Int64 { self.v }
}

struct B: Weighed {
    var v: Int64
    func weight() -> Int64 { self.v * 10 }
}

extend (X, Y) {
    // Element types flow into the signature: must be `(Y, X)`, not unbound.
    func swapped() -> (Y, X) { (self.1, self.0) }
}

extend (X, Y): Weighed where X: Weighed, Y: Weighed {
    func weight() -> Int64 { self.0.weight() + self.1.weight() }
}

@main
func main() -> Int64 {
    let pair = (A(v: 3), B(v: 4));
    if pair.weight() != 43 { return 1 }

    let (b, a) = pair.swapped();
    if b.v != 4 { return 2 }
    if a.v != 3 { return 3 }
    0
}
