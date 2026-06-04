// test: execution
// stdlib: false
// expect-exit: 0

// A conforming type can override a computed property from a protocol extension
// with its own implementation.

module Main

protocol HasCount {
    func count() -> lang.i64
}

extend HasCount {
    var doubleCount: lang.i64 {
        get {
            return lang.i64_mul(self.count(), 2);
        }
    }
}

struct Bag: HasCount {
    var n: lang.i64
    init(n: lang.i64) { self.n = n; }
    func count() -> lang.i64 { return self.n; }

    var doubleCount: lang.i64 {
        get {
            return lang.i64_mul(self.n, 100);
        }
    }
}

struct Box: HasCount {
    var items: lang.i64
    init(items: lang.i64) { self.items = items; }
    func count() -> lang.i64 { return self.items; }
}

@main
func main() -> lang.i64 {
    let b = Bag(3);
    let x = Box(5);
    // Bag overrides: b.doubleCount == 300
    // Box uses default: x.doubleCount == 10
    lang.i64_sub(lang.i64_add(b.doubleCount, x.doubleCount), 310)
}
