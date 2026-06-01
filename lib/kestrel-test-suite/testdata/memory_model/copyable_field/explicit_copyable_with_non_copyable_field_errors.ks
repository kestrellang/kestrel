// test: diagnostics
// stdlib: false

module Test

@builtin(.Copyable)
protocol Copyable {}

struct Resource: not Copyable {
    var fd: lang.i64
}

// Explicitly declares Copyable but contains a non-Copyable field — contradiction.
struct Wrapper: Copyable { // ERROR: contains non-Copyable field
    var r: Resource
}
