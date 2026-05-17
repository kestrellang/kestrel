// test: diagnostics
// stdlib: false

module Test

struct Pair[T] {
    var first: T
    var second: T

    var swapped: (T, T) {
        (self.second, self.first)
    }
}
