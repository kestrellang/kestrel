// test: execution
// stdlib: true
// expect-exit: 0

// A blanket conformance written on a type parameter: `extend[T, R] R:
// Describe[T] where R: Shape` means `extend[T] Shape: Describe[T]`, with `R`
// another name for `Self` — in signatures (`s: R`, `-> R`), in static paths
// (`R.unit()`), and in further bounds (`R: Copyable` becomes `Self:
// Copyable`). It used to be rejected (E452 "cannot extend 'R': unknown
// type").

module Test

protocol Shape {
    func area() -> Int64
    static func unit() -> Self
}

protocol Describe[T] {
    type Out
    static func make(s s: Self, t t: T) -> Out
}

protocol Scale {
    func scaled() -> Int64
    func twin() -> Self
}

struct Sq {
    var n: Int64
}

extend Sq: Shape {
    func area() -> Int64 { self.n * self.n }
    static func unit() -> Sq { Sq(n: 1) }
}

extend[T, R] R: Describe[T] where R: Shape, R: Copyable {
    type Out = Int64
    static func make(s s: R, t t: T) -> Int64 { s.area() + R.unit().area() - 1 }
}

extend[R] R: Scale where R: Shape, R: Copyable {
    func scaled() -> Int64 { self.area() * 10 }
    func twin() -> R { self }
}

func describe[I](i i: I) -> I.Out where I: Describe[Int64] {
    I.make(s: i, t: 0)
}

@main
func main() -> Int64 {
    if Sq.make(s: Sq(n: 4), t: 1) != 16 { return 1 }
    if describe(i: Sq(n: 3)) != 9 { return 2 }
    if Sq(n: 5).twin().n != 5 { return 3 }
    if Sq(n: 2).scaled() != 40 { return 4 }
    0
}
