// test: execution
// stdlib: true
// expect-exit: 1

// Was a `diagnostics` test — which is structurally incapable of catching G18,
// where this program type-checked cleanly and produced the wrong runtime
// answer. Now an `execution` test, so `boolValue()` actually has to run:
// `NonEmpty(count: 0)` has a nonzero-free payload but a `count > 0` truth
// value, so the two branches disagree between a raw branch and correct
// witness dispatch.

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

@main
func main() -> lang.i64 {
    // 3 > 0 -> 1. Also check the false side, where a raw branch on the
    // payload bits would agree by luck: 0 -> 0.
    lang.i64_add(
        test(NonEmpty(count: 3)),
        test(NonEmpty(count: 0)),
    )
}
