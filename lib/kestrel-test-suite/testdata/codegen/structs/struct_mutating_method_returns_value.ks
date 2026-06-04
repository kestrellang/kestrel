// test: execution
// stdlib: true

module Test

struct Counter {
    var value: std.numeric.Int64

    mutating func incrementAndGet(by: std.numeric.Int64) -> std.numeric.Int64 {
        self.value = self.value + by;
        self.value
    }
}

@main
func main() -> lang.i64 {
    var c = Counter(value: 30);
    let result = c.incrementAndGet(12);
    if result != 42 { return 1 }
    if c.value != 42 { return 2 }
    0
}
