// test: execution
// stdlib: true
// expect-exit: 0

module Test

@main
func main() -> lang.i64 {
    let n = 42;
    let s = """
        sum: \(n + 1)
        next: \(n * 2)
        """;
    if s != "sum: 43\nnext: 84" { return 1 }
    0
}
