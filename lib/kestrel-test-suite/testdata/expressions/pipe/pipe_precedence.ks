// test: execution
// stdlib: true
// expect-exit: 0

// `|>` has the lowest binary precedence: the whole left side is piped.

module Test

func double(x: Int64) -> Int64 { x * 2 }
func isBig(x: Int64) -> Bool { x > 10 }

@main
func main() -> lang.i64 {
    // f(a + b)
    if (1 + 2 |> double) != 6 { return 1 }
    if (2 * 3 + 1 |> double) != 14 { return 2 }
    // f(x ?? d)
    let none: Int64? = .None;
    if (none ?? 4 |> double) != 8 { return 3 }
    // f(a or b)
    if not (false or true |> { it }) { return 4 }
    // comparison on the left
    if (20 |> isBig) != true { return 5 }
    // parenthesized pipe inside arithmetic
    if (3 |> double) + 1 != 7 { return 6 }
    0
}
