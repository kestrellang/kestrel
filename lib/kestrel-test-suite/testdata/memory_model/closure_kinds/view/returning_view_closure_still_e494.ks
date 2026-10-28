// test: diagnostics
// stdlib: false

// docs/design/closures.md, Diagnostics table, E494: "unchanged for view kinds".
// A normal-kind closure holds views into the frame, so it can never be returned
// — the fix is an owning kind in the RETURN TYPE (`escaping`/`consuming`), which
// the diagnostic's new fix-it suggests. Distinct trigger from the parameter-
// capture case in expressions/closures/closure_returned_from_function.ks: here
// the capture is a local `var` that was written before the literal.
module Test

func makeCounterView() -> () -> lang.i64 {
    var count = 0;
    count = lang.i64_add(count, 1);
    { count } // ERROR(E494)
}
