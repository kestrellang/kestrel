// test: diagnostics
// stdlib: false

// A `mutating self` method can be called on a call-result receiver.
// The returned value is an owned mutable place — the caller owns it for
// the duration of the call, so mutating it is fine. This is what enables
// chains like `arr.iter().map(f).collect()` without binding to a var.

module Test

struct Counter { var n: lang.i64 }

extend Counter {
    mutating func reset() {
        self.n = 0;
    }
}

func make() -> Counter {
    Counter(n: 1)
}

func test() {
    // Receiver is a function-call result — accepted.
    make().reset();

    // Receiver is an init-call result — accepted.
    Counter(n: 1).reset();
}
