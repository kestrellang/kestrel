// test: diagnostics
// stdlib: true

module Main

// Mirrors `dead_code_in_nested_if.ks`, but nested inside a `for` body so the
// whole if/else divergence analysis has to run below the Sugar wrapper.
func test(b: std.core.Bool) -> std.numeric.Int64 {
    for i in std.core.Range[std.numeric.Int64](0, 5) {
        if b {
            return 1;
        } else {
            return 2;
        }
        let x: std.numeric.Int64 = 3; // WARN: unreachable
    }
    0
}
