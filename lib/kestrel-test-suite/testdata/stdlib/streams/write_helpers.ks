// test: execution
// stdlib: true

// `write(byte:)`, `write(text:)`, `write(line:)`, `write(formatted:)` and
// `writeAll` on any `Writable`.

module Test

import std.streams.(Buffer)

@main
func main() -> lang.i64 {
    var buf = Buffer();
    if buf.write(byte: 65).isErr { return 1 }
    if buf.count != 1 { return 2 }

    var buf2 = Buffer();
    if buf2.write(text: "Hello").isErr { return 3 }
    if buf2.toText() != .Some("Hello") { return 4 }

    var buf3 = Buffer();
    if buf3.write(line: "Hi").isErr { return 5 }
    if buf3.toText() != .Some("Hi\n") { return 6 }

    var buf4 = Buffer();
    let data: Array[UInt8] = [1, 2, 3];
    if buf4.writeAll(from: data.asSlice()).isErr { return 7 }
    let arr = buf4.toArray();
    if arr.count != 3 or arr(0) != 1 or arr(2) != 3 { return 8 }

    var buf5 = Buffer();
    if buf5.write(text: "").isErr { return 9 }
    if buf5.count != 0 { return 10 }

    var buf6 = Buffer();
    let _ = buf6.write(text: "Hello");
    let _ = buf6.write(text: " ");
    let _ = buf6.write(formatted: 42);
    if buf6.toText() != .Some("Hello 42") { return 11 }
    0
}
