// test: execution
// stdlib: true

module Test

import std.text.(CharIndex, GraphemeIndex, LineIndex)

func main() -> lang.i64 {
    // ---- GraphemeIndex.advance ----

    // ASCII: each grapheme is 1 byte
    let s: std.text.String = "hello";
    let slice = s.asSlice();
    let gi0 = GraphemeIndex(0);

    let gi1 = gi0.advance(by: 1, from: slice);
    if gi1.byteOffset != 1 { return 1 }

    let gi3 = gi0.advance(by: 3, from: slice);
    if gi3.byteOffset != 3 { return 2 }

    // Advance from a non-zero position
    let gi4 = gi3.advance(by: 1, from: slice);
    if gi4.byteOffset != 4 { return 3 }

    // Advance past end clamps to end
    let giEnd = gi0.advance(by: 10, from: slice);
    if giEnd.byteOffset != 5 { return 4 }

    // Multi-byte: "héllo" — 'é' is 2 bytes
    let multi: std.text.String = "héllo";
    let mSlice = multi.asSlice();
    let mg0 = GraphemeIndex(0);

    // Advance 1 grapheme: skip 'h' (1 byte)
    let mg1 = mg0.advance(by: 1, from: mSlice);
    if mg1.byteOffset != 1 { return 10 }

    // Advance 2 graphemes: skip 'h' (1) + 'é' (2) = byte 3
    let mg2 = mg0.advance(by: 2, from: mSlice);
    if mg2.byteOffset != 3 { return 11 }

    // Advance from after 'é': skip 'l' (1 byte)
    let mg3 = mg2.advance(by: 1, from: mSlice);
    if mg3.byteOffset != 4 { return 12 }

    // ---- LineIndex.advance ----

    // LF terminators
    let lines: std.text.String = "a\nb\nc";
    let lSlice = lines.asSlice();
    let li0 = LineIndex(0);

    // Advance 1 line: past "a\n" (2 bytes)
    let li1 = li0.advance(by: 1, from: lSlice);
    if li1.byteOffset != 2 { return 20 }

    // Advance 2 lines: past "a\nb\n" (4 bytes)
    let li2 = li0.advance(by: 2, from: lSlice);
    if li2.byteOffset != 4 { return 21 }

    // Advance from non-zero position
    let li2b = li1.advance(by: 1, from: lSlice);
    if li2b.byteOffset != 4 { return 22 }

    // CRLF terminators
    let crlf: std.text.String = "a\r\nb\r\nc";
    let crlfSlice = crlf.asSlice();
    let cl0 = LineIndex(0);

    // Advance 1 line: past "a\r\n" (3 bytes)
    let cl1 = cl0.advance(by: 1, from: crlfSlice);
    if cl1.byteOffset != 3 { return 30 }

    // Advance 2 lines: past "a\r\nb\r\n" (6 bytes)
    let cl2 = cl0.advance(by: 2, from: crlfSlice);
    if cl2.byteOffset != 6 { return 31 }

    // CR-only terminators
    let cr: std.text.String = "a\rb\rc";
    let crSlice = cr.asSlice();
    let cr0 = LineIndex(0);

    let cr1 = cr0.advance(by: 1, from: crSlice);
    if cr1.byteOffset != 2 { return 32 }

    // Advance past end
    let liEnd = li0.advance(by: 10, from: lSlice);
    if liEnd.byteOffset != 5 { return 33 }

    0
}
