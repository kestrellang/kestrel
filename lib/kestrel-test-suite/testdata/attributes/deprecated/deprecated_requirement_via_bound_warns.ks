// test: diagnostics
// stdlib: false

module Test

protocol Shape {
    @deprecated("use 'area()'")
    func size() -> lang.i64
    func area() -> lang.i64
}

// Implementing a deprecated requirement does not warn.
struct Square: Shape {
    func size() -> lang.i64 { 1 }
    func area() -> lang.i64 { 1 }
}

func measure[S]() where S: Shape {}

func viaBound[S](s: S) -> lang.i64 where S: Shape {
    s.size() // WARN: 'size' is deprecated: use 'area()'
}

// Calling the conforming type's own method directly is not a use of the
// requirement.
func direct(s: Square) -> lang.i64 { s.size() }
