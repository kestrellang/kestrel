// test: diagnostics
// stdlib: false

// Comma-chained conditions (`if a, b`) mean the same thing as `a and b`, and
// must report the same way when `LogicalAndOperator` cannot be resolved.
//
// `lower_if_conditions` used to build the `logicalAnd` call by hand instead of
// going through the blessed `SHORT_CIRCUIT_OP_PROTOCOLS` row, and on failure it
// returned the *bare LHS* — dropping the second condition from the program with
// no diagnostic at all (fragility audit F30). Without the stdlib there is no
// `LogicalAndOperator` to resolve, so this file is the failure path.

module Main

func test(a: lang.i1, b: lang.i1) -> lang.i1 {
    if a, b { // ERROR: unsupported binary operator 'and'
        a
    } else {
        b
    }
}
