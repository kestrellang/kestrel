// test: diagnostics
// stdlib: false

module Test

struct Bad {
    var value: lang.i64

    init() {
        self.init(nonexistent: 42) // ERROR: no method 'init' on type 'Bad'
    }
}
