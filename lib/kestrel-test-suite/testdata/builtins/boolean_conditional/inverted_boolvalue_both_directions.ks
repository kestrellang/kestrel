// test: execution
// stdlib: true
// expect-exit: 42

// G18: every condition position used to branch on the condition value's raw
// bits, never calling `BooleanConditional.boolValue()`. This is the shape that
// catches it in BOTH directions:
//
//   - `boolValue()` is a *derived* condition (`count == 0`), not a stored flag,
//     so it disagrees with the payload's bits.
//   - `tag` is declared BEFORE `count` so "branch on the first field's bits"
//     is caught too.
//   - both truth values are exercised: `count = 5` (raw says then, the witness
//     says else) and `count = 0` (raw says else, the witness says then).

module Test

struct EmptyMarker: BooleanConditional {
    var tag: lang.i64
    var count: lang.i64

    func boolValue() -> lang.i1 {
        lang.i64_eq(self.count, 0)
    }
}

@main
func main() -> lang.i64 {
    let five = EmptyMarker(tag: 1, count: 5);
    let zero = EmptyMarker(tag: 1, count: 0);
    // Correct dispatch: 40 + 2 == 42. A raw branch picks 100 and/or 200.
    let a: lang.i64 = if five { 100 } else { 40 };
    let b: lang.i64 = if zero { 2 } else { 200 };
    lang.i64_add(a, b)
}
