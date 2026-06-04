// test: execution
// stdlib: true
// expect-exit: 0

// Regression: when a value's type chains through string interpolation →
// bytes subscript → associated type projection, arg-position integer
// literals in a comparison must infer from the operator's parameter type,
// not default to Int64.
//
// The string interpolation accumulator is blocked by InterpolationLink
// at strict blocking level, which cascades: the bytes subscript result
// stays unresolved → the == Member constraint defers → the integer
// literal is arg-position-blocked. Graduated relaxation (level 1) lifts
// InterpolationLink blocking first, letting the chain resolve before
// the integer literal defaults.

module Test

@main
func main() -> lang.i64 {
    let n: Int64 = 42;
    let s = "\(n)";
    var i: Int64 = 0;
    let b = s.bytes(unchecked: i);
    if b == 52 {   // '4' = 0x34 = 52
        return 0
    }
    1
}
