// test: execution
// stdlib: true

module Test

struct Counter {
    var value: std.numeric.Int64
}

func increment(mutating c: Counter, by: std.numeric.Int64) {
    c.value = c.value + by;
}

@main
func main() -> lang.i64 {
    var c = Counter(value: 0);
    increment(c, 42);
    if c.value != 42 { return 1 }
    0
}
