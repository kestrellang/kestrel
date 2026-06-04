// test: execution
// stdlib: true

module Test

func factorial(n: std.numeric.Int64) -> std.numeric.Int64 {
    if n <= 1 {
        1
    } else {
        n * factorial(n - 1)
    }
}

@main
func main() -> lang.i64 {
    // 5! = 120
    if factorial(5) != 120 { return 1 }
    0
}
