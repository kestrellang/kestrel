// test: execution
// stdlib: true
// expect-exit: 0

// A blanket conformance written on a type parameter: `extend[T, R] R:
// Describe[T] where R: Shape` should give every `Shape` conformer
// `Describe[T]`, as the protocol-extension form `extend[T] Shape:
// Describe[T]` does. Today the target is rejected (E452 "cannot extend 'R':
// unknown type"), so `Sq.make` is undefined (E132).

module Test

protocol Shape {
    func area() -> Int64
}

protocol Describe[T] {
    type Out
    static func make(s s: Self, t t: T) -> Out
}

struct Sq {
    var n: Int64
}

extend Sq: Shape {
    func area() -> Int64 { self.n * self.n }
}

extend[T, R] R: Describe[T] where R: Shape {
    type Out = Int64
    static func make(s s: R, t t: T) -> Int64 { s.area() }
}

func describe[I](i i: I) -> I.Out where I: Describe[Int64] {
    I.make(s: i, t: 0)
}

@main
func main() -> Int64 {
    if Sq.make(s: Sq(n: 4), t: 1) != 16 { return 1 }
    if describe(i: Sq(n: 3)) != 9 { return 2 }
    0
}
