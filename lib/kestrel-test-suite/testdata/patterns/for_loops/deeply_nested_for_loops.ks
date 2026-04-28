// test: diagnostics
// stdlib: true

module Main

func test() {
    var sum: std.numeric.Int64 = 0;
    for i in std.core.Range[std.numeric.Int64](0, 3) {
        for j in std.core.Range[std.numeric.Int64](0, 3) {
            for k in std.core.Range[std.numeric.Int64](0, 3) {
                sum = sum + 1
            }
        }
    }
}
