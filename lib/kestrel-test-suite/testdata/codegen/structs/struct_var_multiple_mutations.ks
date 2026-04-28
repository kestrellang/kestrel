// test: execution
// stdlib: true

module Test

struct Counter {
    var value: std.numeric.Int64
}

func main() -> lang.i64 {
    var c = Counter(value: 0);
    c.value = 10;
    c.value = c.value + 20;
    c.value = c.value + 12;
    if c.value != 42 { return 1 }
    0
}
