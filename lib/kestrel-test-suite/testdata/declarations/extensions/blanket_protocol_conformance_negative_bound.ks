// test: execution
// stdlib: true
// expect-exit: 0

// A blanket conformance written as a protocol extension (`extend[T] Shape:
// Describe[T]`) whose where-clause carries a negative bound on its own param
// (`T: not Static`). The conformer `Sq: Shape` gets `Describe[T]` through it.
// The `not` bound used to fail entailment (only positive bounds checked the
// extension's own where-clause), hiding every witness: a false E454/E455.
// The stdlib's `extend[V] SeqRange: ViewIndex[V] where V: not Static` needs it.

module Test

protocol Shape {
    func area() -> Int64
}

protocol Tag {}

struct Tg {}
extend Tg: Tag {}

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

extend[T] Shape: Describe[T] where T: Tag, T: not Static {
    type Out = Int64
    static func make(s s: Self, t t: T) -> Int64 { s.area() }
}

func describe[I, T](i i: I, t t: T) -> I.Out where I: Describe[T] {
    I.make(s: i, t: t)
}

@main
func main() -> Int64 {
    if Sq.make(s: Sq(n: 4), t: Tg()) != 16 { return 1 }
    if describe(i: Sq(n: 3), t: Tg()) != 9 { return 2 }
    0
}
