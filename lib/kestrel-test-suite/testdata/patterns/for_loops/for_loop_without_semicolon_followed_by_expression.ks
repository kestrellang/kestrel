// test: diagnostics
// stdlib: true

module Main

func test() -> std.numeric.Int {
    var count: std.numeric.Int = 0;
    for i in std.core.Range[std.numeric.Int](0, 5) {
        count = count + 1
    }
    count
}
