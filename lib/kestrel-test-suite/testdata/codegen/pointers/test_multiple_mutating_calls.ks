// test: execution
// stdlib: true

module Test

func add(mutating n: std.num.Int64, amount: std.num.Int64) {
    n = n + amount;
}

func main() -> lang.i64 {
    var x: std.num.Int64 = 0;
    add(x, 10);
    add(x, 20);
    add(x, 12);
    if x != 42 { return 1 }
    0
}
