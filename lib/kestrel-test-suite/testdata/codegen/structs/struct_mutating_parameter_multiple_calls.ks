// test: execution
// stdlib: true

module Test

struct Counter {
    var value: std.num.Int64
}

func increment(mutating c: Counter, by: std.num.Int64) {
    c.value = c.value + by;
}

func main() -> lang.i64 {
    var c = Counter(value: 0);
    increment(c, 10);
    increment(c, 20);
    increment(c, 12);
    if c.value != 42 { return 1 }
    0
}
