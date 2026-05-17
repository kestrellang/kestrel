// test: diagnostics
// stdlib: true

module Main

func test() {
    let arr = std.collections.Array[std.numeric.Int64]();
    var count: std.numeric.Int64 = 0;
    for item in arr {
        count = count + 1
    }
}
