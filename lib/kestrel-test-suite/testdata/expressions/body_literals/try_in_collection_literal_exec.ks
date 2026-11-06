// test: execution
// stdlib: true
// expect-exit: 0
//
// A `try` element in an array or dictionary literal splits the block, which
// threads the literal's buffer through the new blocks under new names. The
// literal used to keep writing through the stale pre-split buffer id: an
// OSSA "use of consumed value" ICE.

module Test

import std.numeric.(Int64)
import std.text.(String)
import std.collections.(Array, Dictionary)
import std.result.(Result)

struct Parser {
    var pos: Int64;

    mutating func next() -> Result[Int64, String] {
        self.pos = self.pos + 1;
        if self.pos > 100 { return .Err("too far"); }
        .Ok(self.pos)
    }

    mutating func word() -> Result[String, String] {
        let n = try self.next();
        .Ok("w\(n)")
    }

    mutating func parse() -> Result[Array[Int64], String] {
        var out: [Int64] = [try self.next()];
        let mixed: [Int64] = [10, try self.next(), 20, try self.next()];
        for x in mixed { out.append(x); }
        let words: [String] = ["a", try self.word(), "b"];
        if words(1) != "w4" { return .Err("bad word") }
        let d: [String: Int64] = ["x": try self.next(), "y": 7];
        out.append(d("x") ?? -1);
        .Ok(out)
    }
}

@main
func main() -> Int64 {
    var p = Parser(pos: 0);
    guard let .Ok(out) = p.parse() else { return 1 }
    if out.count != 6 { return 2 }
    if out(0) != 1 or out(1) != 10 or out(2) != 2 or out(4) != 3 { return 3 }
    if out(5) != 5 { return 4 }
    // The failure path leaves the literal half built and must unwind it.
    var q = Parser(pos: 99);
    if q.parse().isOk { return 5 }
    0
}
