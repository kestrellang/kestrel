// test: diagnostics
// stdlib: true

module Test
struct NonEmpty: BooleanConditional {
    var count: lang.i64

    func boolValue() -> lang.i1 {
        lang.i64_signed_gt(self.count, 0)
    }
}
func test(items: NonEmpty) -> lang.i64 {
    if items {
        1
    } else {
        0
    }
}
