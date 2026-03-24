// test: execution
// stdlib: true

module Test

func incrementAndGet(mutating n: std.num.Int64) -> std.num.Int64 {
    n = n + 1;
    n
}

func main() -> lang.i64 {
    var x: std.num.Int64 = 41;
    let result = incrementAndGet(x);
    if result != 42 { return 1 }
    0
}
