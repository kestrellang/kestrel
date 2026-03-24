// test: diagnostics
// stdlib: true

module Main

func test() -> std.num.Int {
    var sum: std.num.Int = 0;
    for i in std.core.Range[std.num.Int](1, 6) {
        sum = sum + i
    }
    sum
}
