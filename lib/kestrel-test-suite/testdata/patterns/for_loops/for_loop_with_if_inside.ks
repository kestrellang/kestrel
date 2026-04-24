// test: diagnostics
// stdlib: true

module Main

func test() {
    var evenSum: std.num.Int64 = 0;
    var oddSum: std.num.Int64 = 0;
    for i in std.core.Range[std.num.Int64](0, 10) {
        if i % 2 == 0 {
            evenSum = evenSum + i
        } else {
            oddSum = oddSum + i
        }
    }
}
