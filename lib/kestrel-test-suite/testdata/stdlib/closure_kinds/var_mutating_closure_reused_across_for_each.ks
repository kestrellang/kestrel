// test: execution
// stdlib: true
// expect-exit: 0

// A pre-existing `mutating` closure value is not Copyable and its calls are
// exclusive, so it must be held in a `var` (docs/design/closures.md,
// "mutating: write-back"). Pins that one such `var` is reusable across repeated
// `forEach` calls and keeps accumulating into the same captured frame place.
module Test

@main
func main() -> lang.i64 {
    var total: Int64 = 0;

    // The kind is spelled in the type — there is no kind-on-literal syntax.
    var accumulate: mutating (Int64) -> () = { (x) in total = total + x };

    [1, 2, 3].iter().forEach(accumulate);
    if total != 6 { return 1 }

    // Reused, not consumed: passing to a `mutating` parameter is an exclusive
    // borrow of the `var`, not a move.
    [4, 5].iter().forEach(accumulate);
    if total != 15 { return 2 }

    // Reading the frozen place while the closure is still live is legal —
    // the freeze only blocks destruction, not reads or plain assignment.
    total = total + 1;
    [10].iter().forEach(accumulate);
    if total != 26 { return 3 }

    0
}
