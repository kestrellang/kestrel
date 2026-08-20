// test: diagnostics
// stdlib: true

// A `guard` chain of two plain boolean conditions. Comma-chained conditions
// mean `a and b`, so this needs the stdlib's `And` conformance on `Bool` to
// resolve; the stdlib-less failure path is covered by
// `expressions/short_circuit/comma_conditions_report_missing_and_operator.ks`.

module Main

func test(a: Bool, b: Bool) -> lang.i64 {
    guard a, b else {
        return 0
    }
    42
}
