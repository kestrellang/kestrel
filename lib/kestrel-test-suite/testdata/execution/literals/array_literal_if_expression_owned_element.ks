// test: execution
// stdlib: true
// expect-exit: 0

// An array literal whose element is an `if` expression yielding an owned
// value (here a String). Today this crashes the OSSA verifier ("use of
// consumed value") in the enclosing function. The same `if` as a tuple
// element, or a plain String literal as the array element, compiles fine.
// Found building the title screen in examples/aerie.

module Test

func pick(flag: Bool) -> String {
    let xs: [String] = ["A", if flag { "B" } else { "C" }];
    xs(1)
}

@main
func main() -> lang.i64 {
    if pick(true) != "B" { return 1; }
    if pick(false) != "C" { return 2; }
    0
}
