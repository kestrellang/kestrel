// test: execution
// stdlib: true

// `readByte`, `readAll`, `readExact` and `readText` on any `Readable`.

module Test

import std.streams.(Cursor, Empty, IoErrorKind)

@main
func main() -> lang.i64 {
    var cursor = Cursor(bytes: [65, 66, 67]);
    match cursor.readByte() { .Ok(.Some(b)) => if b != 65 { return 1 }, _ => return 2 }
    match cursor.readByte() { .Ok(.Some(b)) => if b != 66 { return 3 }, _ => return 4 }
    match cursor.readByte() { .Ok(.Some(b)) => if b != 67 { return 5 }, _ => return 6 }
    match cursor.readByte() { .Ok(.None) => (), _ => return 7 }

    // readAll appends everything left.
    var cursor2 = Cursor(bytes: [1, 2, 3]);
    var dest = Array[UInt8]();
    match cursor2.readAll(into: dest) {
        .Ok(n) => if n != 3 { return 8 },
        .Err(_) => return 9
    }
    if dest.count != 3 or dest(0) != 1 or dest(2) != 3 { return 10 }

    var empty = Empty();
    var emptyDest = Array[UInt8]();
    match empty.readAll(into: emptyDest) {
        .Ok(n) => if n != 0 { return 11 },
        .Err(_) => return 12
    }

    // readExact fails with UnexpectedEnd when the stream runs out.
    var short = Cursor(bytes: [1, 2]);
    var four = Array[UInt8](repeating: 0, count: 4);
    match short.readExact(into: four.asMutableSlice()) {
        .Ok(_) => return 13,
        .Err(e) => if e.kind != IoErrorKind.UnexpectedEnd { return 14 }
    }

    // readText decodes UTF-8 and rejects invalid bytes.
    var text = Cursor(text: "héllo");
    match text.readText() { .Ok(t) => if t != "héllo" { return 15 }, .Err(_) => return 16 }
    var bad = Cursor(bytes: [255, 254]);
    match bad.readText() {
        .Ok(_) => return 17,
        .Err(e) => if e.kind != IoErrorKind.InvalidData { return 18 }
    }
    0
}
