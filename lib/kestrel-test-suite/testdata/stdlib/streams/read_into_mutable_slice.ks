// test: execution
// stdlib: true
// expect-exit: 0

// `Readable.read(into:)` takes a `mutating Slice[UInt8]`: a reader can
// only write the elements the caller handed it, never past them, and a
// write never reaches storage another array shares (the raw view runs the
// copy-on-write check first). Moved from std.io.

module Test

import std.collections.(Array)
import std.streams.(Cursor, Repeat)

@main
func main() -> lang.i64 {
    var buf = Array[UInt8](repeating: 0, count: 6);
    let shared = buf;
    // A sub-slice: only elements 2..<4 change.
    var rep = Repeat(byte: 7);
    match rep.read(into: buf(2..<4)) {
        .Ok(n) => if n != 2 { return 1 },
        .Err(_) => return 2
    }
    if buf(1) != 0 or buf(2) != 7 or buf(3) != 7 or buf(4) != 0 { return 3 }
    // Copy-on-write: the copy taken before the read is untouched.
    if shared(2) != 0 { return 4 }
    // readExact loops a short reader across a reborrowed tail.
    var cur = Cursor(bytes: [1, 2, 3, 4, 5, 6]);
    match cur.readExact(into: buf.asMutableSlice()) {
        .Ok(_) => {},
        .Err(_) => return 5
    }
    if buf(0) != 1 or buf(5) != 6 { return 6 }
    if shared(5) != 0 { return 7 }
    0
}
