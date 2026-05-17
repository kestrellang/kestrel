// test: diagnostics
// stdlib: true

module Main

func test() {
    var evenSum: std.numeric.Int64 = 0;
    var oddSum: std.numeric.Int64 = 0;
    for i in std.core.Range[std.numeric.Int64](0, 10) {
        if i % 2 == 0 {
            evenSum = evenSum + i
        } else {
            oddSum = oddSum + i
        }
    }
}
