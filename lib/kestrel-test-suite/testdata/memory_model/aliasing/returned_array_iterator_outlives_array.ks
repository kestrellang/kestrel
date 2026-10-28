// test: execution
// stdlib: true
// expect-exit: 0

// Returning `a.iter()` for a local array `a`. `Array.iter()` returns an
// `ArrayIterator`, which holds a share of the array's storage, so the buffer
// outlives `a` and the caller iterates both elements. (Found in the 2026-10
// architecture review at 9767d2dc, when the iterator held only a raw pointer:
// segfault.)

module Test

import std.collections.ArrayIterator

func make() -> ArrayIterator[String] {
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
