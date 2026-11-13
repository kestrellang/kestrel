// test: execution
// stdlib: true
// expect-exit: 0
//
// An extension target with a tuple nested in a generic's arguments,
// `extend Box[(A, B)]`, should bind A and B like a top-level `extend (A, B)`
// does. Today the names are unbound (E476 "cannot find type 'A'"); found
// 2026-11-01 writing perch-2's `extend Path[(A, B)]: FromRequest`.

module Test

protocol P { func n() -> Int64 }
extend Int64: P { func n() -> Int64 { self } }

struct Box[T] { var v: T }

extend Box[(A, B)] where A: P, B: P {
    func sum() -> Int64 { self.v.0.n() + self.v.1.n() }
}

@main
func main() -> Int64 {
    let b = Box(v: (3, 4));
    b.sum() - 7
}
