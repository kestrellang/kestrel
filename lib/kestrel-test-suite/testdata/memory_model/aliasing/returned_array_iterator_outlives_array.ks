// test: execution
// stdlib: true
// expect-exit: 0

// Returning `a.iter()` for a local array `a`. `ArraySliceIterator` holds a
// `Pointer` into `a`'s storage with no retain, and `Pointer` is `Static`, so
// the escape checker (E494) never sees the iterator as carrying a view of `a`.
// `a` is dropped at return and the caller iterates freed storage. Found in the
// 2026-10 architecture review at 9767d2dc: segfault (exit 139), valgrind
// invalid read in `ArraySliceIterator.next`.
//
// Expected semantics: the iterator keeps the storage alive and yields both
// elements. (If the fix is instead to reject the return with E494, this
// becomes a diagnostics test.)
// EXPECTED TO FAIL until array iterators keep their storage alive.

module Test

import std.memory.ArraySliceIterator

func make() -> ArraySliceIterator[String] {
    let a = [
        "first heap-allocated string, long enough to avoid inline storage",
        "second heap-allocated string, long enough to avoid inline storage",
    ];
    a.iter()
}

@main
func main() -> lang.i64 {
    var it = make();
    var junk = Array[String]();
    var i = 0;
    while i < 64 {
        junk.append("filler \(i) padded out so it reuses the freed buffer");
        i = i + 1;
    }
    var seen = 0;
    var bytes = 0;
    while let .Some(s) = it.next() {
        bytes = bytes + s.bytes.count;
        seen = seen + 1;
    }
    if seen != 2 { return 1; }
    if bytes != 129 { return 2; }
    0
}
