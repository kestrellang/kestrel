// test: diagnostics
// stdlib: false

module Main

struct Counter {
    var count: lang.i64
    
    mutating func add(n: lang.i64) {
        self.count = lang.i64_add(self.count, n);
    }
}
