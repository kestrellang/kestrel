// test: execution
// stdlib: true

module Test

struct Counter {
    var value: std.numeric.Int64

    mutating func increment(by: std.numeric.Int64) {
        self.value = self.value + by;
    }
}

func double_increment(mutating c: Counter, amount: std.numeric.Int64) {
    c.increment(amount);
    c.increment(amount);
}

@main
func main() -> lang.i64 {
    var c = Counter(value: 0);
    double_increment(c, 21);
    if c.value != 42 { return 1 }
    0
}
