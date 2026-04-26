// test: diagnostics
// stdlib: false

module Main

struct Counter {
    var count: lang.i64
    
    init(start: lang.i64) {
        self.count = start;
    }
}
