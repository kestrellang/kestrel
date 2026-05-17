// test: diagnostics
// stdlib: true

module Main

func test() {
    var result = std.collections.Array[std.numeric.Int64]();
    for i in std.core.Range[std.numeric.Int64](0, 5) {
        result.append(i * 2)
    }
}
