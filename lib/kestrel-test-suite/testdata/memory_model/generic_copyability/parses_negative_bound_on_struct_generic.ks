// test: diagnostics
// stdlib: true

module Test

struct Box[T] where T: not Copyable {
    var value: T
}
