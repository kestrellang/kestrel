// test: execution
// stdlib: true
// expect-exit: 0

// An untyped literal on the LEFT of a binary operator cannot take its type from
// the right operand: `100 + small` (small: Int8) and `2.0 * b` (b: Float32)
// are rejected with E100 "type mismatch", while the mirrored `small + 100` and
// `b * 2.0` are accepted. The literal-default blocked set only protects
// literals in argument position (solver.rs `apply_literal_defaults`), so the
// receiver literal is force-defaulted to Int64 / Float64 before the operator
// links it to the other operand. Found in the 2026-10 architecture review at
// 9767d2dc.
// EXPECTED TO FAIL until operator constraints let either operand fix a
// literal's type.

module Test

@main
func main() -> lang.i64 {
    let small: Int8 = 10;
    let a = small + 100;
    let b = 100 + small;
    if a != 110 { return 1; }
    if b != 110 { return 2; }

    let f: Float32 = 1.5;
    let c = f * 2.0;
    let d = 2.0 * f;
    if c != 3.0 { return 3; }
    if d != 3.0 { return 4; }
    0
}
