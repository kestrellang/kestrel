// test: diagnostics
// stdlib: true

module Main

func test() {
    var count: std.num.Int64 = 0;
    for i in std.core.Range[std.num.Int64](0, 100) {
        count = count + 1;
        if i > 5 {
            break
        }
    }
}
