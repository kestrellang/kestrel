// test: execution
// stdlib: true
// expect-exit: 0

// A `mutating` protocol-extension method marked `internal` or `fileprivate`
// keeps its mutating receiver at call sites. Lowering used to look the
// method up visibility-filtered from the root module, miss it, and pass the
// receiver as a borrow (ICE: witness convention mismatch).

module Test

protocol Source {
    mutating func pull() -> Int64
}

extend Source {
    internal mutating func pullTwice() -> Int64 {
        self.pull() + self.pull()
    }

    fileprivate mutating func pullThrice() -> Int64 {
        self.pull() + self.pull() + self.pull()
    }
}

struct Counter: Source {
    var n: Int64
    mutating func pull() -> Int64 {
        self.n = self.n + 1;
        self.n
    }
    mutating func viaSelf() -> Int64 { self.pullTwice() }
}

@main
func main() -> lang.i64 {
    var c = Counter(n: 0);
    if c.pullTwice() != 3 { return 1 }      // 1 + 2
    if c.viaSelf() != 7 { return 2 }        // 3 + 4
    if c.pullThrice() != 18 { return 3 }    // 5 + 6 + 7
    if c.n != 7 { return 4 }
    0
}
