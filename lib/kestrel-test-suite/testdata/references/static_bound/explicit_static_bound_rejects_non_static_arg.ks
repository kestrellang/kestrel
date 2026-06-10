// test: diagnostics
// stdlib: false

// An EXPLICIT `where T: Static` bound (no implicit injection yet in this
// commit) rejects a `not Static` argument and accepts a plain one.

module Test

@builtin(.Copyable)
protocol Copyable {}

@builtin(.Static)
protocol Static {}

struct Handle: not Static {
    var id: lang.i64
}

func requireStatic[T](x: T) where T: Static {}

func ok(n: lang.i64) {
    requireStatic(n);
}

func bad(h: Handle) {
    requireStatic(h); // ERROR: !: Static
}
