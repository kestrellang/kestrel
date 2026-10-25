// test: diagnostics
// stdlib: true

// Regression: calling a missing method whose name collides with a
// non-function field must report the missing *method* against the
// *receiver* — not misfire as a subscript on the field's type.
//
// `ArraySlice` has a `count` property and an internal `len: Int64` field
// but no `len()` method. `sl.len()` used to resolve to the `len` field,
// forward `solve_call(Int64, [])`, and report
//   "no matching subscript on type 'Int64'"
// (wrong kind, wrong type). It must instead report against ArraySlice.
//
// `len` is declared `fileprivate`, so the receiver-side error names that.
// The old parser misplaced the `]` of the preceding `Pointer[T]` field into
// `len`'s Visibility node, the modifier was lost, and the field read as
// public — this test previously expected "no member 'len'" for that reason.

module Test

@main
func main() -> lang.i64 {
    var arr = std.collections.Array[std.numeric.Int64]();
    arr.append(10); arr.append(20); arr.append(30);
    let sl = arr(0..<2);
    sl.len() // ERROR: member 'len' is fileprivate and not accessible from this scope
}
