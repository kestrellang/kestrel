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

module Test

@main
func main() -> lang.i64 {
    var arr = std.collections.Array[std.numeric.Int64]();
    arr.append(10); arr.append(20); arr.append(30);
    let sl = arr(0..<2);
    sl.len() // ERROR: no member 'len' on type 'ArraySlice[Int64]'
}
