// test: diagnostics
// stdlib: false

module Test

struct Box {
    var width: lang.i64
    var height: lang.i64
    var depth: lang.i64

    init(width width: lang.i64, height height: lang.i64, depth depth: lang.i64) {
        self.width = width;
        self.height = height;
        self.depth = depth
    }

    init(side: lang.i64) {
        self.init(width: side, height: side, depth: side)
    }

    init() {
        self.init(1)
    }
}
