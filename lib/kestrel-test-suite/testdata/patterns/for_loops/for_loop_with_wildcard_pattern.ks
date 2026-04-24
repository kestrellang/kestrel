// test: diagnostics
// stdlib: true

module Main

func test() {
    var count: std.num.Int64 = 0;
    for _ in std.core.Range[std.num.Int64](0, 10) {
        count = count + 1
    }
}
