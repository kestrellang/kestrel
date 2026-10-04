// test: execution
// stdlib: true
// expect-exit: 0

// Appending to an array while `for`-iterating it. `for` desugars to
// `let $iter = arr.iter()` (hir-lower desugar.rs), and `Array.iter()` builds
// `ArraySliceIterator(ptr: self.ptr(), ...)` with no retain on the storage
// (array.ks). The appends reallocate the buffer and the iterator keeps reading
// the freed one. Found in the 2026-10 architecture review at 9767d2dc: the
// second iteration yields a garbage element, valgrind reports an invalid read
// in `ArraySliceIterator.next`.
//
// Expected semantics (Swift's `IndexingIterator`): the iterator sees the
// array's value at the start of the loop — exactly 1, 2, 3.
// EXPECTED TO FAIL until array iterators keep their storage alive.

module Test

@main
func main() -> lang.i64 {
    var arr = [1, 2, 3];
    var sum = 0;
    var iterations = 0;
    for x in arr {
        arr.append(x * 10);
        arr.append(x * 100);
        arr.append(x * 1000);
        sum = sum + x;
        iterations = iterations + 1;
    }
    if iterations != 3 { return 1; }
    if sum != 6 { return 2; }
    if arr.count != 12 { return 3; }
    0
}
