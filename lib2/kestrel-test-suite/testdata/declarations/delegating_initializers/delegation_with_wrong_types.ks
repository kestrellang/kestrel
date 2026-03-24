// test: diagnostics
// stdlib: false

module Test

struct Bad {
    var value: lang.i64

    init(value: lang.i64) {
        self.value = value
    }

    init(text: lang.str) {
        self.init(value: text) // ERROR: type
    }
}
