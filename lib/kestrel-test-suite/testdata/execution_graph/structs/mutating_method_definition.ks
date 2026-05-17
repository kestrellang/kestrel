// test: diagnostics
// stdlib: false

module Main

struct Counter {
    var count: lang.i64
    
    mutating func increment() {
        self.count = lang.i64_add(self.count, 1);
    }
}
