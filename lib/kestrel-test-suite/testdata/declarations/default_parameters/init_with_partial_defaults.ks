// test: diagnostics
// stdlib: false

module Main

struct Rectangle {
    var width: lang.i64
    var height: lang.i64

    init(width: lang.i64, height: lang.i64 = 100) {
        self.width = width;
        self.height = height;
    }
}

func test() -> Rectangle {
    Rectangle(50)
}
