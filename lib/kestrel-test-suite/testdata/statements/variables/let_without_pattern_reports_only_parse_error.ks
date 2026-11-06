// test: diagnostics

// A `let` whose pattern failed to parse reports only the parse error. It used
// to destructure an internal temporary, and follow-on errors leaked its name
// ("access to uninitialized variable '$let_tmp'").

module Test

func f() -> Int64 {
    let = 1; // ERROR: expected pattern
    2
}
