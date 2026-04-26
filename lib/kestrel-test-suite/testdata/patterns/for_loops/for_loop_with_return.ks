// test: diagnostics
// stdlib: true

module Main

func test() -> std.num.Int {
    for i in std.core.Range[std.num.Int](0, 100) {
        if i > 50 {
            return i
        }
    }
    0
}
