// test: diagnostics
// stdlib: true

module Test

struct Container {
    var count: lang.i64

    init[T](items: [T]) {
        self.count = 0
    }
}
