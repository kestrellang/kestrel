// test: diagnostics
// stdlib: false

// Static bound failures carry a "because" detail naming the cause —
// here the explicit `not Static` declaration.

module Test

@builtin(.Copyable)
protocol Copyable {}

@builtin(.Static)
protocol Static {}

struct Handle: not Static {
    var id: lang.i64
}

func requireStatic[T](x: T) where T: Static {}

func bad(h: Handle) {
    requireStatic(h); // ERROR: declared 'not Static'
}
