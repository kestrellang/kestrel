// test: diagnostics
// stdlib: false

module Main

struct Point {
    var x: lang.i64
    var y: lang.i64

    init(quick: lang.i1) {
        if quick {
            self.x = 0;
            self.y = 0;
            return;
        }
        self.x = 1;
        self.y = 2;
    }
}
