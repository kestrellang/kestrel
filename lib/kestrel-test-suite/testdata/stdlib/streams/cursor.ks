// test: execution
// stdlib: true

// `Cursor` reads from bytes in memory and seeks within them.

module Test

import std.streams.(Cursor, Seek)

@main
func main() -> lang.i64 {
    var cursor = Cursor(bytes: [10, 20, 30, 40, 50]);
    if cursor.position != 0 or cursor.remaining != 5 { return 1 }

    var buf = Array[UInt8](repeating: 0, count: 3);
    match cursor.read(into: buf.asMutableSlice()) {
        .Ok(n) => if n != 3 { return 2 },
        .Err(_) => return 3
    }
    if buf(0) != 10 or buf(1) != 20 or buf(2) != 30 { return 4 }
    if cursor.position != 3 { return 5 }

    // A short read at the end.
    var buf2 = Array[UInt8](repeating: 0, count: 5);
    match cursor.read(into: buf2.asMutableSlice()) {
        .Ok(n) => if n != 2 { return 6 },
        .Err(_) => return 7
    }
    if buf2(0) != 40 or buf2(1) != 50 { return 8 }

    // End of stream reads 0.
    match cursor.read(into: buf.asMutableSlice()) {
        .Ok(n) => if n != 0 { return 9 },
        .Err(_) => return 10
    }

    // Setting the position.
    cursor.position = 1;
    var buf3 = Array[UInt8](repeating: 0, count: 2);
    match cursor.read(into: buf3.asMutableSlice()) {
        .Ok(n) => if n != 2 { return 11 },
        .Err(_) => return 12
    }
    if buf3(0) != 20 or buf3(1) != 30 { return 13 }
    cursor.position = -5;
    if cursor.position != 0 { return 14 }

    // Seeking: relative to the end, and before the start is an error.
    match cursor.seek(to: .End(-1)) {
        .Ok(at) => if at != 4 { return 15 },
        .Err(_) => return 16
    }
    if cursor.seek(to: .Current(-10)).isOk { return 17 }

    let text = Cursor(text: "hi");
    if text.remaining != 2 { return 18 }
    0
}
