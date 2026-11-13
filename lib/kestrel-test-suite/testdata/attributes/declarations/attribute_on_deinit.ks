// test: diagnostics
// stdlib: false

module Test

@builtin(.Copyable)
protocol Copyable {}

struct S: not Copyable {
    @dummy
    deinit {}
}
