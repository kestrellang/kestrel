// test: diagnostics
// stdlib: false

module Test

struct Box {
    var value: lang.i64

    init[T](value: lang.i64) {
        self.value = value
    }
}
