// test: diagnostics
// stdlib: false

module Test

struct Box[T] {
    var value: T

    var contents: T {
        self.value
    }
}
