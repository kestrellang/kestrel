// test: diagnostics
// stdlib: false

module Test
struct Number: Prelude.BooleanConditional {
    var value: lang.i64

    func asBool() -> lang.i1 {
        lang.i64_ne(self.value, 0)
    }
}
