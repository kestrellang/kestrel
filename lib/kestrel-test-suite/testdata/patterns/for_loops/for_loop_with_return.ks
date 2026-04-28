// test: diagnostics
// stdlib: true

module Main

func test() -> std.numeric.Int {
    for i in std.core.Range[std.numeric.Int](0, 100) {
        if i > 50 {
            return i
        }
    }
    0
}
