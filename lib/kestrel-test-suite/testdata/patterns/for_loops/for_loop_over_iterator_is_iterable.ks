// test: diagnostics
// stdlib: true

module Main

import std.iter.Iterator

struct Counter: Iterator {
    var current: std.num.Int64
    var end: std.num.Int64

    type Item = std.num.Int64

    init(end end: std.num.Int64) {
        self.current = 0;
        self.end = end;
    }

    mutating func next() -> std.result.Optional[std.num.Int64] {
        if self.current < self.end {
            let value = self.current;
            self.current = self.current + 1;
            .Some(value)
        } else {
            .None
        }
    }
}

func test() {
    var sum: std.num.Int64 = 0;
    var counter = Counter(end: 3);
    for x in counter {
        sum = sum + x
    }
}
