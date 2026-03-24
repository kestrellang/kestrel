// test: diagnostics
// stdlib: false

module Main

struct Point {
    var x: lang.i64
    var y: lang.i64

    init() { // ERROR
        self.x = 0;
    }
}
