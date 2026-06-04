// test: execution
// stdlib: false
// expect-exit: 0

// Computed property in protocol extension calls a protocol method and
// another extension method, verifying self-dispatch chains correctly.

module Main

protocol HasCount {
    func count() -> lang.i64
}

extend HasCount {
    func tripleCount() -> lang.i64 {
        return lang.i64_mul(self.count(), 3);
    }

    var sixCount: lang.i64 {
        get {
            return lang.i64_mul(self.tripleCount(), 2);
        }
    }
}

struct Bag: HasCount {
    var n: lang.i64
    init(n: lang.i64) { self.n = n; }
    func count() -> lang.i64 { return self.n; }
}

@main
func main() -> lang.i64 {
    let b = Bag(4);
    // sixCount == 4 * 3 * 2 == 24
    lang.i64_sub(b.sixCount, 24)
}
