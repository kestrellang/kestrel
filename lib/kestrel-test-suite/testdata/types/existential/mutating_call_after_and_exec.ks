// test: execution
// stdlib: true
// expect-exit: 0
//
// A mutating requirement called on `any P` borrows the box mutably to reach
// its payload. That borrow used to stay open until scope exit, so a later
// read of the box in the same block — the closure `and` builds for its
// right side captures it — failed OSSA verification with "active mut
// borrow". The borrow now ends with the call.

module Test

import std.numeric.(Int64)
import std.text.(String)
import std.result.(Result)

protocol Sink {
    var total: Int64 { get }
    mutating func take(n: Int64)
}

struct Counter: Sink {
    var total: Int64;
    mutating func take(n: Int64) { self.total = self.total + n; }
}

protocol Decoder {
    var name: String { get }
    mutating func write(n: Int64, mutating into sink: any Sink) -> Result[Int64, String]
    mutating func finish(mutating into sink: any Sink) -> Result[Int64, String]
}

struct Simple: Decoder {
    var name: String;
    var count: Int64;
    mutating func write(n: Int64, mutating into sink: any Sink) -> Result[Int64, String] {
        sink.take(n);
        self.count = self.count + 1;
        .Ok(self.count)
    }
    mutating func finish(mutating into sink: any Sink) -> Result[Int64, String] {
        sink.take(100);
        self.count = self.count + 1;
        .Ok(self.count)
    }
}

@main
func main() -> Int64 {
    var decoder: any Decoder = Simple(name: "simple", count: 0);
    var sink: any Sink = Counter(total: 0);
    let status = decoder.write(3, into: sink);
    if not (status.isOk and decoder.finish(into: sink).isOk) { return 1 }
    if sink.total != 103 { return 2 }
    // Short-circuits: `finish` must not run.
    if status.isErr and decoder.finish(into: sink).isOk { return 3 }
    if sink.total != 103 { return 4 }
    if not (status.isErr or decoder.finish(into: sink).isOk) { return 5 }
    if sink.total != 203 { return 6 }
    if not (decoder.name == "simple" and decoder.write(1, into: sink).isOk) { return 7 }
    if sink.total != 204 { return 8 }
    0
}
