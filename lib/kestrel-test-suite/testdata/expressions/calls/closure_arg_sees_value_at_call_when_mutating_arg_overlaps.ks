// test: execution
// stdlib: true
// expect-exit: 0

// Fuzz H12: a read-only closure argument that captures a place also passed
// as `mutating` (argument or receiver) in the same call sees the value AT
// THE CALL — the same rule as a plain borrowed argument (`plain(x, x)`).
// Covers either argument order, field captures, a mutating receiver, and a
// String (refcounted) capture. Used to ICE: "active mut borrow".
module H

import std.numeric.(Int64)
import std.text.(String)

func two(mutating a: Int64, f: () -> Int64) -> Int64 {
    a = a + 1;
    a + f()
}

func rev(f: () -> Int64, mutating a: Int64) -> Int64 {
    a = a + 10;
    a + f()
}

func plain(mutating a: Int64, b: Int64) -> Int64 {
    a = a + 1;
    a + b
}

struct P {
    var a: Int64
    var b: Int64
    mutating func bump(f: () -> Int64) -> Int64 {
        self.a = self.a + 100;
        self.a + f()
    }
}

func strs(mutating s: String, f: () -> Bool) -> Bool {
    s = s + "xyz";
    f()
}

@main
func main() -> Int64 {
    var x: Int64 = 1;
    // f sees x at the call (1); a becomes 2.
    if two(x, { x }) != 3 { return 1 }
    if x != 2 { return 2 }
    // mutating arg after the closure.
    if rev({ x }, x) != 14 { return 3 }
    if x != 12 { return 4 }
    // plain-argument rule, for comparison.
    if plain(x, x) != 25 { return 5 }
    var p = P(a: 1, b: 2);
    // field capture and mutating receiver.
    if p.bump({ p.a + p.b }) != 104 { return 6 }
    if p.a != 101 { return 7 }
    var s: String = "ab";
    // f sees the call-time "ab"; s itself becomes "abxyz".
    if strs(s, { s == "ab" }) == false { return 8 }
    if s != "abxyz" { return 9 }
    0
}
