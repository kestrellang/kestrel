// test: diagnostics
// stdlib: true

module Main

// Same as the `break` case: `continue` diverges inside the desugared loop.
func test() {
    for i in std.core.Range[std.numeric.Int64](0, 5) {
        continue;
        let x: std.numeric.Int64 = 1; // WARN: unreachable
    }
}
