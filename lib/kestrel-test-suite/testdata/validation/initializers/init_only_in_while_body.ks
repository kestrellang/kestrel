// test: diagnostics
// stdlib: false

module Main

struct Counter {
    var value: lang.i64

    init(cond: lang.i1) {
        while cond {
            self.value = 0;
        }
    } // ERROR: does not initialize all fields
}
