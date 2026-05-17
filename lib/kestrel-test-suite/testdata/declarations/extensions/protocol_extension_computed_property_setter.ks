// test: execution
// stdlib: false
// expect-exit: 0

// Computed property with getter and setter in a protocol extension.

module Main

protocol HasCount {
    func count() -> lang.i64
    func setCount(to value: lang.i64)
}

extend HasCount {
    var doubleCount: lang.i64 {
        get {
            return lang.i64_mul(self.count(), 2);
        }
        set {
            self.setCount(to: newValue);
        }
    }
}

struct Bag: HasCount {
    var n: lang.i64
    init(n: lang.i64) { self.n = n; }
    func count() -> lang.i64 { return self.n; }
    mutating func setCount(to value: lang.i64) { self.n = value; }
}

func main() -> lang.i64 {
    var b = Bag(3);
    let before = b.doubleCount;  // 6
    b.doubleCount = 10;
    let after = b.count();  // 10 (setter passes newValue through)
    // 6 + 10 - 16 == 0
    lang.i64_sub(lang.i64_add(before, after), 16)
}
