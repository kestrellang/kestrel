// test: diagnostics
// stdlib: false

// Forming `Wrap[Handle]` in a type annotation must be rejected at the
// formation site (the wellformedness check covers Static like Copyable).

module Test

@builtin(.Copyable)
protocol Copyable {}

@builtin(.Static)
protocol Static {}

struct Handle: not Static {
    var id: lang.i64
}

struct Wrap[T] {
    var inner: T
}

func mk(consuming h: Handle) -> Wrap[Handle] { // ERROR: !: Static
    return Wrap(inner: h)
}
