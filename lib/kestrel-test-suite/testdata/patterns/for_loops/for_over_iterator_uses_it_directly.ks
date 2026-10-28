// test: execution
// stdlib: true
// backends: cranelift,llvm
// expect-exit: 0

module Test

// `for` over an Iterator consumes it as the loop's iterator; over any other
// Iterable it calls `iter()`. Covers a non-Copyable iterator local and
// rvalue, an adapter chain, and collections of heap-owning elements.

import std.collections.(Set)

var deinits: Int64 = 0

struct Countdown: Iterator, not Copyable {
    type Item = Int64
    var n: Int64
    mutating func next() -> Int64? {
        if self.n == 0 { return .None }
        self.n -= 1;
        .Some(self.n)
    }
    deinit { deinits += 1; }
}

func sumSet(s: Set[String]) -> Int64 {
    var t = 0;
    for x in s { t += x.bytes.count; }
    t
}

func overLocal() -> Int64 {
    let c = Countdown(n: 3);
    var a = 0;
    for v in c { a += v; }
    a
}

func overRvalue() -> Int64 {
    var b = 0;
    for v in Countdown(n: 4) { b += v; }
    b
}

@main
func main() -> Int64 {
    // The moved iterator is dropped exactly once (checked after the helper
    // returns: the language doesn't pin it to the loop's end).
    if overLocal() != 3 { return 1 }              // 2 + 1 + 0
    if deinits != 1 { return 2 }
    if overRvalue() != 6 { return 3 }
    if deinits != 2 { return 4 }

    let words = ["ab", "cde"];
    var e = 0;
    for (i, w) in words.iter().enumerate() { e += i * 10 + w.bytes.count; }
    if e != 15 { return 5 }

    var s = Set[String]();
    s.insert("xy");
    s.insert("zzz");
    if sumSet(s) != 5 { return 6 }
    if sumSet(s) != 5 { return 7 }               // the set is intact after a loop
    0
}
