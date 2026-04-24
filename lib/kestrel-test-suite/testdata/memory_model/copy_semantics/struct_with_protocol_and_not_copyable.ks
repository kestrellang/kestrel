// test: diagnostics
// stdlib: false

module Test

@builtin(.Copyable)
protocol Copyable {}

protocol Resource {}

struct Handle: Resource, not Copyable {
    var fd: lang.i64
}
