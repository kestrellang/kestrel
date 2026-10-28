// test: execution
// stdlib: true

// Regression for issue #121: an exhaustive tuple-literal `match` with no
// wildcard arm used to fail OSSA verify at build ("@owned value live at block
// exit but never consumed"). The boolean-branch decision-tree path threaded the
// per-element Bool test copies through every successor block but never consumed
// them, so they were live at each leaf with no merge edge. It must now build and
// select the correct arm.
module Test

func f(a: Bool, b: Bool) -> Int64 {
    match (a, b) {
        (true, true) => 0,
        (true, false) => 1,
        (false, true) => 2,
        (false, false) => 3
    }
}

@main
func main() -> lang.i64 {
    if not (f(true, true) == 0) { return 1; }
    if not (f(true, false) == 1) { return 2; }
    if not (f(false, true) == 2) { return 3; }
    if not (f(false, false) == 3) { return 4; }
    return 0;
}
