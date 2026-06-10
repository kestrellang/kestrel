// test: diagnostics
// stdlib: false

// E208: a `&T` place is readable through, never writable — plain
// assignment through a shared-ref-returning call or getter is the
// assignment analyzer's twin of E207 (which covers the compound form).
module Test

struct Counter {
    var n: lang.i64
}

struct Box {
    var c: Counter

    func peek() -> &Counter { self.c }
    mutating func peekMut() -> &mutating Counter { self.c }
}

func use(mutating b: Box) {
    b.peekMut() = Counter(n: 2);
    b.peek() = Counter(n: 3); // ERROR(E208)
}
