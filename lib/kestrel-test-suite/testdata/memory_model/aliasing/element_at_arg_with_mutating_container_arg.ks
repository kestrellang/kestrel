// test: execution
// stdlib: true
// expect-exit: 0

// `g(arr, arr(at: 0))`: the container passed `mutating` and one of its elements
// passed as a borrowed argument through the `at:` accessor. Borrow-convention
// arguments are place contexts (references-gaps.md §10.5), so `b` is a view
// into the heap buffer; `a = []` frees that buffer while `b` is still live.
// Found in the 2026-10 architecture review at 9767d2dc: segfault (exit 139),
// valgrind invalid read in `RcBox.clone`. The getter form `g(arr, arr(0))`
// copies the element and is safe.
//
// Expected: `b` is the element's value at the call.
// EXPECTED TO FAIL until a `mutating` argument may not overlap another argument
// of the same call. If the fix is static rejection, this becomes a
// diagnostics test.

module Test

func g(mutating a: [String], b: String) -> Int64 {
    a = [];
    var junk = Array[String]();
    var i = 0;
    while i < 64 {
        junk.append("filler \(i) padded out so it reuses the freed buffer");
        i = i + 1;
    }
    b.bytes.count
}

@main
func main() -> lang.i64 {
    var arr = ["first heap-allocated string, long enough to avoid inline storage"];
    let n = g(arr, arr(at: 0));
    if n != 64 { return 1; }
    if arr.count != 0 { return 2; }
    0
}
