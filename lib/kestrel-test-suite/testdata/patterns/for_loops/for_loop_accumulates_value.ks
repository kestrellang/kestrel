// test: diagnostics
// stdlib: true

module Main

func test() -> std.numeric.Int {
    var sum: std.numeric.Int = 0;
    for i in std.core.Range[std.numeric.Int](1, 6) {
        sum = sum + i
    }
    sum
}
