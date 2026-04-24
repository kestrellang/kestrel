// test: diagnostics
// stdlib: true

module Main

func test() {
    var sum: std.num.Int64 = 0;
    for var x in std.core.Range[std.num.Int64](0, 5) {
        x = x + 1;
        sum = sum + x
    }
}
