// test: diagnostics
// stdlib: true

module Main

func test() {
    for i in std.core.Range[std.numeric.Int64](0, 5) {
        var j: std.numeric.Int64 = 0;
        while j < i {
            j = j + 1
        }
    }
}
