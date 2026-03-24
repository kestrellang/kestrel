// test: diagnostics
// stdlib: false

module Test
struct Text: Prelude.BooleanConditional {
    var length: lang.i64

    func asBool() -> lang.i1 {
        lang.i64_signed_gt(self.length, 0)
    }
}
