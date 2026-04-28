// test: diagnostics
// stdlib: true

module Main

func test() {
    var count: std.numeric.Int64 = 0;
    while count < 5 {
        for i in std.core.Range[std.numeric.Int64](0, 3) {
            count = count + 1
        }
    }
}
