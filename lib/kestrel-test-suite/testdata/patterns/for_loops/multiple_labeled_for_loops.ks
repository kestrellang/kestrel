// test: diagnostics
// stdlib: true

module Main

func test() {
    outer: for i in std.core.Range[std.numeric.Int64](0, 10) {
        middle: for j in std.core.Range[std.numeric.Int64](0, 10) {
            inner: for k in std.core.Range[std.numeric.Int64](0, 10) {
                if k > 3 {
                    break inner
                }
                if j > 5 {
                    break middle
                }
                if i > 7 {
                    break outer
                }
            }
        }
    }
}
