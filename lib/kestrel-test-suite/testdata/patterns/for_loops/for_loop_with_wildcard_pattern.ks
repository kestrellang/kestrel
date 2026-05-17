// test: diagnostics
// stdlib: true

module Main

func test() {
    var count: std.numeric.Int64 = 0;
    for _ in std.core.Range[std.numeric.Int64](0, 10) {
        count = count + 1
    }
}
