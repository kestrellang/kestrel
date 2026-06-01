// test: diagnostics
// stdlib: false

module Test

@builtin(.Copyable)
protocol Copyable {}

// All fields Copyable — explicit Copyable is fine, no diagnostic.
struct Point: Copyable {
    var x: lang.i64
    var y: lang.i64
}
