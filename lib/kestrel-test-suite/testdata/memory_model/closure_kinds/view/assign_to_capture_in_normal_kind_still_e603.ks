// test: diagnostics
// stdlib: false

// docs/design/closures.md, Diagnostics table, E603 ("kept for normal bodies,
// with a fix-it suggesting `mutating`/`escaping`") and "Inference": an expected
// NORMAL type is not silently upgraded — the annotation `() -> lang.i64` picks
// the view kind, so writing through a capture is still rejected. The diagnostic
// must also carry a fix-it note pointing at a `mutating` closure type.
module Test

func test() -> lang.i64 {
    var total = 0;
    let f: () -> lang.i64 = {
        total = lang.i64_add(total, 1); // ERROR: cannot assign to captured variable 'total'
        total
    };
    f()
}
