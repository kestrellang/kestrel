// test: execution
// stdlib: true

// Regression: an `if`-expression used as a *sub-operand* of a larger
// expression (not the whole RHS of a `let`) ICE'd in MIR-3 OSSA verify
// ("@owned value is live at block exit but never consumed"). The @owned
// operand value materialized before the branch was not threaded through the
// if's blocks. Two shapes: a value before the if (`m - (if ...)`) and two
// if-operands (the first stranded across the second's blocks).
module Test

func compute(m: std.numeric.Int64) -> std.numeric.Int64 {
    let y = m - (if m <= 2 { 1 } else { 0 });
    let z = (if m < 1 { 10 } else { 20 }) - (if m > 3 { 3 } else { 4 });
    y + z
}

@main
func main() -> lang.i64 {
    if compute(1) != 16 { return 1; }   // (1-1) + (20-4)
    if compute(5) != 22 { return 2; }   // (5-0) + (20-3)
    0
}
