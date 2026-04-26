// test: diagnostics
// stdlib: false

module Main

struct Counter {
    var count: lang.i64
    
    mutating func increment() {
        self.count = lang.i64_add(self.count, 1);
    }

    func read() -> lang.i64 {
        self.count
    }
}

func main() -> lang.i64 {
    var c = Counter(count: 0);
    c.increment();
    c.increment();
    c.read()
}
