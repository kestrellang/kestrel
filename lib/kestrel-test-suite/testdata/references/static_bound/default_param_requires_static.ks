// test: diagnostics
// stdlib: false

// The IMPLICIT `T: Static` bound: a plain generic with no where clause
// rejects a `not Static` argument — generic code may store/return its
// params, so containment is the default.

module Test

@builtin(.Copyable)
protocol Copyable {}

@builtin(.Static)
protocol Static {}

struct Handle: not Static {
    var id: lang.i64
}

func identity[T](consuming x: T) -> T {
    x
}

func bad(consuming h: Handle) {
    identity(h); // ERROR: !: Static
}
