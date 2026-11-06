// test: execution
// stdlib: true

// `Repeat` fills any buffer with one byte; `Sink` takes everything.

module Test

import std.streams.(Repeat, Sink)

@main
func main() -> lang.i64 {
    var rep = Repeat(byte: 42);
    var buf = Array[UInt8](repeating: 0, count: 5);
    match rep.read(into: buf.asMutableSlice()) {
        .Ok(n) => if n != 5 { return 1 },
        .Err(_) => return 2
    }
    for b in buf {
        if b != 42 { return 3 }
    }
    var buf2 = Array[UInt8](repeating: 0, count: 2);
    match rep.read(into: buf2.asMutableSlice()) {
        .Ok(n) => if n != 2 { return 4 },
        .Err(_) => return 5
    }

    var sink = Sink();
    let data: Array[UInt8] = [1, 2, 3];
    match sink.write(from: data.asSlice()) {
        .Ok(n) => if n != 3 { return 6 },
        .Err(_) => return 7
    }
    if sink.flush().isErr { return 8 }
    let big = Array[UInt8](repeating: 255, count: 100);
    match sink.write(from: big.asSlice()) {
        .Ok(n) => if n != 100 { return 9 },
        .Err(_) => return 10
    }
    0
}
