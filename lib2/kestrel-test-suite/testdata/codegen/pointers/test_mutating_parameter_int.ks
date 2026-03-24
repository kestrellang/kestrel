// test: execution
// stdlib: true

module Test

func increment(mutating n: std.num.Int64) {
    n = n + 1;
}

func main() -> lang.i64 {
    var x: std.num.Int64 = 41;
    increment(x);
    if x != 42 { return 1 }
    0
}
