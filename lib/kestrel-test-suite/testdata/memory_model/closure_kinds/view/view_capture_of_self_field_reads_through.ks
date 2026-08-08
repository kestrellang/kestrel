// test: execution
// stdlib: false
// expect-exit: 0

// docs/design/closures.md, "What is captured: the narrowest place" + "How it is
// captured": inside a method on a non-Copyable type, `{ self.n }` captures the
// PLACE `self.n` (never the whole `self`), and it captures it by view — so the
// write performed after the closure is created is visible through `f()`.
module Test

struct Counter: not Copyable {
    var n: lang.i64

    mutating func bumpAndRead() -> lang.i64 {
        let f = { self.n };                  // view of the place `self.n`
        self.n = lang.i64_add(self.n, 5);    // plain write-back stays legal
        f()                                  // sees 15, not the pre-write 10
    }
}

@main
func main() -> lang.i64 {
    var c = Counter(n: 10);
    let got = c.bumpAndRead();
    if lang.i64_eq(got, 15) { } else { return 1; }
    if lang.i64_eq(c.n, 15) { } else { return 2; }
    0
}
