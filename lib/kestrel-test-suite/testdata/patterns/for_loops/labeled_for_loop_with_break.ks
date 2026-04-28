// test: diagnostics
// stdlib: true

module Main

func test() {
    outer: for i in std.core.Range[std.numeric.Int64](0, 10) {
        for j in std.core.Range[std.numeric.Int64](0, 10) {
            if j > 5 {
                break outer
            }
        }
    }
}
