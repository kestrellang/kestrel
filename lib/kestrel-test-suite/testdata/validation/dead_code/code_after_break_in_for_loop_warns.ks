// test: diagnostics
// stdlib: true

module Main

// `break` inside a `for` body diverges, so the following statement is dead.
// Requires the analyzer to recurse Sugar -> Block -> Loop (which sets
// `in_loop`) -> Match arm -> user body.
func test() {
    for i in std.core.Range[std.numeric.Int64](0, 5) {
        break;
        let x: std.numeric.Int64 = 1; // WARN: unreachable
    }
}
