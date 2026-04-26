// test: diagnostics
// stdlib: true

module Main

func test() -> std.num.Int {
    var count: std.num.Int = 0;
    for i in std.core.Range[std.num.Int](0, 5) {
        count = count + 1
    }
    count
}
