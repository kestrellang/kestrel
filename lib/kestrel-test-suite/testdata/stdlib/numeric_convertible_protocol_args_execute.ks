// test: execution
// stdlib: true
// expect-exit: 0

// Fuzz H20: conversions whose protocol args match a declared conformance
// still type-check and run, including UInt64.random(in:) (its body used a
// UInt64 -> UInt64 conversion the new check caught).
module H

import std.numeric.(Int8, Int16, Int32, Int64, UInt8, UInt64)

@main
func main() -> Int64 {
    let a: Int8 = 5;
    let b = Int16(from: a);
    let c = Int8(from: b);
    let d = Int32(from: c);
    let s: Int16 = 7;
    let e = Int8(from: s);
    let u: UInt8 = 3;
    let f = Int64(from: u);
    var rng = Lcg64(seed: 1);
    let r = UInt64.random(in: 2..<3, using: rng);
    Int64(from: d) + Int64(from: e) + f + 4 + Int64(from: r) - 21
}
