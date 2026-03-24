// test: diagnostics
// stdlib: true

module Main

func test() {
    var sum: std.num.Int64 = 0;
    for i in std.core.Range[std.num.Int64](0, 5) {
        sum = sum + i
    }
    for j in std.core.Range[std.num.Int64](0, 3) {
        sum = sum + j
    }
}
