// test: diagnostics
// stdlib: false

module Test
struct Counter: Prelude.BooleanConditional {
    var remaining: lang.i64

    func asBool() -> lang.i1 {
        lang.i64_signed_gt(self.remaining, 0)
    }

    mutating func decrement() {
        self.remaining = lang.i64_sub(self.remaining, 1)
    }
}
func countdown() {
    var c = Counter(remaining: 5);
    while c {
        c.decrement();
    }
}
