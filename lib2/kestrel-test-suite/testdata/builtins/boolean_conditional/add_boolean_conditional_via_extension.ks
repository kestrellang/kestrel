// test: diagnostics
// stdlib: false

module Test
struct Status {
    var code: lang.i64
}
extend Status: Prelude.BooleanConditional {
    func asBool() -> lang.i1 {
        lang.i64_eq(self.code, 0)
    }
}
func test(s: Status) -> lang.i64 {
    if s {
        1
    } else {
        0
    }
}
