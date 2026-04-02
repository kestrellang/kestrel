// test: diagnostics
// stdlib: true

module Test
struct Number: BooleanConditional {
    var value: lang.i64

    func boolValue() -> lang.i1 {
        lang.i64_ne(self.value, 0)
    }
}
