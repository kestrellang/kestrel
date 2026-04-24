// test: diagnostics
// stdlib: false

// A `mutating self` method on a let-bound receiver must still fail.
// The receiver-position loosening only covers owned temporaries; a named
// immutable binding is not an owned place — it's a read of someone else's
// place — so mutation through it is rejected.

module Test

struct Counter { var n: lang.i64 }

extend Counter {
    mutating func reset() {
        self.n = 0;
    }
}

func test() {
    let c = Counter(n: 1);
    c.reset(); // ERROR: immutable binding 'c'
}
