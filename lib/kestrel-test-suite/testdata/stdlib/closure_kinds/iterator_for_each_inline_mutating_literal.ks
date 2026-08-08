// test: execution
// stdlib: true
// expect-exit: 0

// Pins `Iterator.forEach` as a `mutating`-closure API (docs/design/closures.md,
// "mutating: write-back" + the stdlib audit's five eager side-effect APIs).
// An inline literal in a `mutating` parameter position captures `&mutating`
// views, so assignments in the body write back to the enclosing frame vars.
module Test

@main
func main() -> lang.i64 {
    // Single captured accumulator — the written-back value is the assertion.
    var total: Int64 = 0;
    [1, 2, 3, 4].iter().forEach({ (x) in total = total + x });
    if total != 10 { return 1 }

    // Two independent captured places in one literal: overlapping-place
    // merging never collapses them into a single capture.
    var seen: Int64 = 0;
    var last: Int64 = 0;
    [5, 6, 7].iter().forEach({ (x) in seen = seen + 1; last = x; });
    if seen != 3 { return 2 }
    if last != 7 { return 3 }

    // Empty source: the action never runs, so the captured place is untouched.
    var untouched: Int64 = 42;
    let empty = Array[Int64]();
    empty.iter().forEach({ (x) in untouched = 0 });
    if untouched != 42 { return 4 }

    0
}
