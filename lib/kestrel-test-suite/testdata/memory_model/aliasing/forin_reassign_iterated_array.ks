// test: execution
// stdlib: true
// expect-exit: 0

// Reassigning an array while `for`-iterating it. The iterator holds a raw
// pointer into the storage with no retain (`Array.iter()`, array.ks), so
// `arr = []` frees the buffer the loop is still reading. Found in the 2026-10
// architecture review at 9767d2dc: segfault (exit 139).
//
// Expected semantics: the iterator sees the array's value at the start of the
// loop, so both original strings are visited.
// EXPECTED TO FAIL until array iterators keep their storage alive.

module Test

@main
func main() -> lang.i64 {
    var arr = [
        "first heap-allocated string, long enough to avoid inline storage",
        "second heap-allocated string, long enough to avoid inline storage",
    ];
    var seen = 0;
    var bytes = 0;
    for s in arr {
        arr = [];
        var junk = Array[String]();
        var i = 0;
        while i < 64 {
            junk.append("filler \(i) padded out so it reuses the freed buffer");
            i = i + 1;
        }
        bytes = bytes + s.bytes.count;
        seen = seen + 1;
    }
    if seen != 2 { return 1; }
    if bytes != 129 { return 2; }
    if arr.count != 0 { return 3; }
    0
}
