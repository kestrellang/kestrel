// test: execution
// stdlib: true
// expect-exit: 0

// A right side without `_` is evaluated, then called with the left value.

module Test

func makeAdder(n: Int64) -> escaping (Int64) -> Int64 {
    { it + n }
}

@main
func main() -> lang.i64 {
    // a call with no `_` returns the function that is called
    if (5 |> makeAdder(1)) != 6 { return 1 }
    // closure literal
    if (4 |> { it * 2 }) != 8 { return 2 }
    if (4 |> { (x) in x - 1 }) != 3 { return 3 }
    // a function-typed local
    let triple: (Int64) -> Int64 = { it * 3 };
    if (4 |> triple) != 12 { return 4 }
    // parenthesized right side is just an expression
    if (4 |> (makeAdder(10))) != 14 { return 5 }
    0
}
