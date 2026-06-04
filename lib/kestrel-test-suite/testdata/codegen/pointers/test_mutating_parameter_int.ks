// test: execution
// stdlib: true

module Test

func increment(mutating n: std.numeric.Int64) {
    n = n + 1;
}

@main
func main() -> lang.i64 {
    var x: std.numeric.Int64 = 41;
    increment(x);
    if x != 42 { return 1 }
    0
}
