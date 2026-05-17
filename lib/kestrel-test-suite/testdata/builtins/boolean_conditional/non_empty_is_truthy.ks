// test: diagnostics
// stdlib: true

module Test
struct Text: BooleanConditional {
    var length: lang.i64

    func boolValue() -> lang.i1 {
        lang.i64_signed_gt(self.length, 0)
    }
}
