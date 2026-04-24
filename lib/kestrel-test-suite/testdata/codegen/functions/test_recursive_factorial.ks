// test: execution
// stdlib: true

module Test

func factorial(n: std.num.Int64) -> std.num.Int64 {
    if n <= 1 {
        1
    } else {
        n * factorial(n - 1)
    }
}

func main() -> lang.i64 {
    // 5! = 120
    if factorial(5) != 120 { return 1 }
    0
}
