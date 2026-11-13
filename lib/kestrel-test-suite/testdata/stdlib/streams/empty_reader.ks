// test: execution
// stdlib: true

// `Empty` is always at end of stream.

module Test

import std.streams.(Empty)

@main
func main() -> lang.i64 {
    var empty = Empty();
    var buf = Array[UInt8](repeating: 0, count: 3);
    match empty.read(into: buf.asMutableSlice()) {
        .Ok(n) => if n != 0 { return 1 },
        .Err(_) => return 2
    }
    match empty.read(into: buf.asMutableSlice()) {
        .Ok(n) => if n != 0 { return 3 },
        .Err(_) => return 4
    }
    0
}
