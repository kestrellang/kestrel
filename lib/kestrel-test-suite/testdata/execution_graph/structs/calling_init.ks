// test: diagnostics
// stdlib: false

module Main

struct Counter {
    var count: lang.i64
    
    init(start: lang.i64) {
        self.count = start;
    }

    func read() -> lang.i64 {
        self.count
    }
}

func main() -> lang.i64 {
    let c = Counter(42);
    c.read()
}
