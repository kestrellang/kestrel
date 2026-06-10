// test: diagnostics
// stdlib: false

// E-REF-20: mutating use through a SHARED `&T` is the const-cast error —
// a mutating method, `mutating`-param argument, compound assign, or setter
// base needs `&mutating T`. The `&mutating` line below carries NO
// annotation: it also pins that E205's "temporary receiver" wording does
// not fire for mutable-ref call results (they classify Mutable, not
// Temporary).
module Test

struct Counter {
    var n: lang.i64
    mutating func bump() { self.n = 1; }
}

struct Box {
    var c: Counter

    func peek() -> &Counter { self.c }
    mutating func peekMut() -> &mutating Counter { self.c }
}

func use(mutating b: Box) {
    b.peekMut().bump();
    b.peek().bump(); // ERROR(E207)
}
