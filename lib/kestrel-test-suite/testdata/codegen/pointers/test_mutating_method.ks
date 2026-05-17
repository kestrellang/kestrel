// test: execution
// stdlib: true

module Test

struct Counter {
    var count: std.numeric.Int64

    mutating func increment() {
        self.count = self.count + 1;
    }

    func read() -> std.numeric.Int64 {
        self.count
    }
}

func main() -> lang.i64 {
    var c = Counter(count: 40);
    c.increment();
    c.increment();
    if c.read() != 42 { return 1 }
    0
}
