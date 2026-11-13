// test: execution
// stdlib: true
// expect-exit: 0

// A BufferedReader can borrow its stream (`&mutating R: Readable`), and
// `innerMutable()` reaches the stream to reply on it: a request/response
// loop without giving the stream up. Both the reads and the replies land
// on the caller's `Duplex`.
module Test
import std.streams.(Readable, Writable, IoError, BufferedReader)
import std.collections.(Array, Slice)

// An in-memory duplex stream: reads from `input`, records writes.
struct Duplex: Readable, Writable, not Copyable {
    var input: Array[UInt8]
    var pos: Int64
    var output: Array[UInt8]

    public mutating func read(mutating into buf: Slice[UInt8]) -> Result[Int64, IoError] {
        var n: Int64 = 0;
        while n < buf.count and self.pos < self.input.count {
            buf(n) = self.input(self.pos);
            n = n + 1;
            self.pos = self.pos + 1
        }
        .Ok(n)
    }
    public mutating func write(from buf: Slice[UInt8]) -> Result[Int64, IoError] {
        for b in buf { self.output.append(b) }
        .Ok(buf.count)
    }
    public mutating func flush() -> Result[(), IoError] { .Ok(()) }
}

func serve(mutating stream: Duplex) -> Result[Int64, IoError] {
    let s = &mutating stream;
    var reader = BufferedReader(s);
    var handled: Int64 = 0;
    loop {
        guard let .Some(line) = try reader.readLine() else { break }
        try reader.innerMutable().write(line: "echo \(line)");
        handled = handled + 1
    }
    .Ok(handled)
}

@main
func main() -> Int64 {
    var d = Duplex(input: Array(from: "a\nb\n".bytes), pos: 0, output: Array());
    let n = match serve(d) { .Ok(n) => n, .Err(_) => -1 };
    let out = String(fromUtf8: d.output.asSlice()) ?? "?";
    print("[\(n)] [\(out)]\n");
    if n != 2 { return 1 }
    if out != "echo a\necho b\n" { return 2 }
    0
}
