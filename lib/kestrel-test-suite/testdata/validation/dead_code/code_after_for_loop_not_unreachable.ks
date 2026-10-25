// test: diagnostics
// stdlib: true

module Main

// NEGATIVE: pins `expr_diverges(Sugar{ForLoop}) == false`. The desugared loop
// exits via the `.None => break` arm, so `block_contains_break` is true and the
// loop does not diverge — code after the `for` stays reachable.
func test() -> std.numeric.Int64 {
    var total: std.numeric.Int64 = 0;
    for i in std.core.Range[std.numeric.Int64](0, 5) {
        total = total + i;
    }
    total
}
