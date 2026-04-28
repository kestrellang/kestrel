// test: execution
// stdlib: true

module Test

struct Counter {
    var value: std.numeric.Int64

    mutating func increment(by: std.numeric.Int64) {
        self.value = self.value + by;
    }
}

func main() -> lang.i64 {
    var c = Counter(value: 0);
    c.increment(42);
    if c.value != 42 { return 1 }
    0
}
