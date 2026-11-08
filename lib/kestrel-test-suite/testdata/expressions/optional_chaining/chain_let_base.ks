// test: diagnostics
// stdlib: true

module Test

struct Counter {
    var n: Int64
    mutating func bump() { self.n = self.n + 1; }
}

func test() {
    let c: Counter? = .Some(Counter(n: 1));
    c?.bump(); // ERROR(E203)
    c?.n = 2; // ERROR(E203)
}
