// test: diagnostics
// stdlib: true

module Main

func test() {
    var sum: std.numeric.Int64 = 0;
    for i in std.core.Range[std.numeric.Int64](0, 10) {
        if i == 5 {
            continue
        }
        sum = sum + i
    }
}
