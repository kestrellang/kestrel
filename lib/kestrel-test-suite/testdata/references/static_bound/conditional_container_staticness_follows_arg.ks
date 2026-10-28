// test: diagnostics
// stdlib: false

// Per-instantiation staticness: `Box[T] where T: not Static { var v: T }`
// is Static iff its arg is — Box[i64] passes a Static bound, Box[Handle]
// fails it, with the offending type argument named in the detail.

module Test

@builtin(.Copyable)
protocol Copyable {}

@builtin(.Static)
protocol Static {}

struct Handle: not Static {
    var id: lang.i64
}

struct Box[T] where T: not Static {
    var v: T
}

func requireStatic[T](consuming x: T) where T: Static {}

func ok(n: lang.i64) {
    requireStatic(Box(v: n));
}

func bad(consuming h: Handle) {
    requireStatic(Box(v: h)); // ERROR: type argument 'Handle' is non-Static
}
