// test: diagnostics
// stdlib: true

module Test

struct Wrapper {
    var value: lang.i64

    init[T](items items: [T]) {
        self.value = 0
    }

    init[K, V](pairs pairs: [(K, V)]) {
        self.value = 0
    }
}
