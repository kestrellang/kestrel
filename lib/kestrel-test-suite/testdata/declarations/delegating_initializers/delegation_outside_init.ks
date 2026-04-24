// test: diagnostics
// stdlib: false

module Test

struct Bad {
    var value: lang.i64

    init(value: lang.i64) {
        self.value = value
    }

    func reset() {
        self.init(value: 0) // ERROR: init
    }
}
