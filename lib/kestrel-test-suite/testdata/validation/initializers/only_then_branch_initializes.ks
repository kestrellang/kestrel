// test: diagnostics
// stdlib: false

module Main

struct Point {
    var x: lang.i64

    init(cond: lang.i1) {
        if cond {
            self.x = 1;
        }
    } // ERROR: does not initialize all fields
}
