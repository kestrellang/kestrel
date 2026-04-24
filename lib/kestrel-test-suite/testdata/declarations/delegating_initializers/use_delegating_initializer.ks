// test: diagnostics
// stdlib: false

module Test

struct Rect {
    var width: lang.i64
    var height: lang.i64

    init(width: lang.i64, height: lang.i64) {
        self.width = width;
        self.height = height
    }

    init(size: lang.i64) {
        self.init(size, size)
    }
}

func test() -> Rect {
    Rect(10)
}
