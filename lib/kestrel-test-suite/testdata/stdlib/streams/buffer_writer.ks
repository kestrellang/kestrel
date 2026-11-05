// test: execution
// stdlib: true

// `Buffer` collects written bytes in memory. Replaces std.io's
// io_buffer_writer test.

module Test

import std.streams.(Buffer)

@main
func main() -> lang.i64 {
    var buf = Buffer();
    if not buf.isEmpty { return 1 }
    if buf.count != 0 { return 2 }

    let data: Array[UInt8] = [72, 101, 108, 108, 111];
    match buf.write(from: data.asSlice()) {
        .Ok(n) => if n != 5 { return 3 },
        .Err(_) => return 4
    }
    if buf.count != 5 { return 5 }
    if buf.isEmpty { return 6 }
    if buf.toText() != .Some("Hello") { return 7 }
    let sl = buf.asSlice();
    if sl.count != 5 or sl(0) != 72 { return 8 }
    if buf.flush().isErr { return 9 }

    let more: Array[UInt8] = [33];
    match buf.write(from: more.asSlice()) {
        .Ok(n) => if n != 1 { return 10 },
        .Err(_) => return 11
    }
    if buf.toText() != .Some("Hello!") { return 12 }
    let arr = buf.clone().toArray();
    if arr.count != 6 or arr(5) != 33 { return 13 }

    buf.clear();
    if buf.count != 0 or not buf.isEmpty { return 14 }

    let sized = Buffer(capacity: 64);
    if not sized.isEmpty { return 15 }
    0
}
