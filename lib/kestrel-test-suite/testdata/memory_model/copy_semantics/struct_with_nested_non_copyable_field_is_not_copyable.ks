// test: diagnostics
// stdlib: false

module Test

@builtin(.Copyable)
protocol Copyable {}

struct Handle: not Copyable {
    var fd: lang.i64
}

struct Inner {
    var handle: Handle
}

struct Outer {
    var inner: Inner
}
