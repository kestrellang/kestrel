// test: diagnostics
// stdlib: true

// Regression: calling a missing method whose name collides with a
// non-function field must report the missing *method* against the
// *receiver* — not misfire as a subscript on the field's type.
//
// `Slice` has a `count` property and an internal `elements` field but no
// `elements()` method. A call like `sl.elements()` used to resolve to the
// field, forward `solve_call(<field type>, [])`, and report
//   "no matching subscript on type ..."
// (wrong kind, wrong type). It must instead report against Slice.
//
// `elements` is declared `fileprivate`, so the receiver-side error names
// that. (Before `Slice`, this test used `ArraySlice`'s `len` field, which an
// old parser bug once made public.)

module Test

@main
func main() -> lang.i64 {
    var arr = std.collections.Array[std.numeric.Int64]();
    arr.append(10); arr.append(20); arr.append(30);
    let sl = &arr(0..<2);
    sl.elements() // ERROR: member 'elements' is fileprivate and not accessible from this scope
}
