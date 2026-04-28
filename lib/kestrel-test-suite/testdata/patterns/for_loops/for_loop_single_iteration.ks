// test: diagnostics
// stdlib: true

module Main

func test() {
    var count: std.numeric.Int64 = 0;
    for i in std.core.Range[std.numeric.Int64](0, 1) {
        count = count + 1
    }
}
