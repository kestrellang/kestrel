// test: execution
// stdlib: true

module Test

func incrementAndGet(mutating n: std.numeric.Int64) -> std.numeric.Int64 {
    n = n + 1;
    n
}

func main() -> lang.i64 {
    var x: std.numeric.Int64 = 41;
    let result = incrementAndGet(x);
    if result != 42 { return 1 }
    0
}
