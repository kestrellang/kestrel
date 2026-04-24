// test: execution
// stdlib: true

module Test

struct Counter {
    var value: std.num.Int64
}

func main() -> lang.i64 {
    var c = Counter(value: 0);
    c.value = 42;
    if c.value != 42 { return 1 }
    0
}
