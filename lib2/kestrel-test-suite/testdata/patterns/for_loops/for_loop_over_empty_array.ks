// test: diagnostics
// stdlib: true

module Main

func test() {
    let arr = std.collections.Array[std.num.Int64]();
    var count: std.num.Int64 = 0;
    for item in arr {
        count = count + 1
    }
}
