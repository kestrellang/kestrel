// test: diagnostics
// stdlib: false

// Structural propagation needs NO declaration: a struct storing a
// non-Static field is itself non-Static, and the detail names the field
// (mirrors NotCopyable child propagation).

module Test

@builtin(.Copyable)
protocol Copyable {}

@builtin(.Static)
protocol Static {}

struct Handle: not Static {
    var id: lang.i64
}

struct Holder {
    var h: Handle
}

func requireStatic[T](consuming x: T) where T: Static {}

func bad(consuming holder: Holder) {
    requireStatic(holder); // ERROR: field 'h' is non-Static
}
