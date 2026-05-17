// test: diagnostics
// stdlib: true

module Test
struct Status {
    var code: lang.i64
}
extend Status: BooleanConditional {
    func boolValue() -> lang.i1 {
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
