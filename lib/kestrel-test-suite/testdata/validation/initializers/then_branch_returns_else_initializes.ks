// test: diagnostics
// stdlib: false

module Main

struct Point {
    var x: lang.i64

    init(cond: lang.i1) {
        if cond {
            self.x = 1;
            return;
        }
        self.x = 2;
    }
}
