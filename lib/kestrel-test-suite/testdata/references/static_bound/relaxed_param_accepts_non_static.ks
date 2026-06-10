// test: diagnostics
// stdlib: false

// `where T: not Static` relaxes the implicit bound: the param accepts BOTH
// non-Static and Static arguments (need-not, the `not Copyable`
// convention). Inside the relaxed body, T is not provably Static, so
// passing it onward to a Static-defaulted generic fails — containment.

module Test

@builtin(.Copyable)
protocol Copyable {}

@builtin(.Static)
protocol Static {}

struct Handle: not Static {
    var id: lang.i64
}

func relaxed[T](consuming x: T) where T: not Static {}

func relaxedForward[T](consuming x: T) where T: not Static {
    identity(x); // ERROR: !: Static
}

func identity[T](consuming x: T) -> T {
    x
}

func ok(consuming h: Handle, n: lang.i64) {
    relaxed(h);
    relaxed(n);
}
