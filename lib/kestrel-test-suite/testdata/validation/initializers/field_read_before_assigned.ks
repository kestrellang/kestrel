// test: diagnostics
// stdlib: false

module Main

struct Point {
    var x: lang.i64
    var y: lang.i64

    init() {
        self.y = self.x; // ERROR: before it is initialized
        self.x = 0;
    }
}
