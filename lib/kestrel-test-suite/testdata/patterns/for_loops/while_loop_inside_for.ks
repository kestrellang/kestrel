// test: diagnostics
// stdlib: true

module Main

func test() {
    for i in std.core.Range[std.num.Int64](0, 5) {
        var j: std.num.Int64 = 0;
        while j < i {
            j = j + 1
        }
    }
}
