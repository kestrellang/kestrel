// test: diagnostics
// stdlib: true

// Fuzz H20: a call resolved through a generic protocol's member must check
// the protocol's ARGS, not just the protocol. `Int16` conforms to
// `Convertible[Int8]`, `[UInt8]` and `[Int16]` but not `Convertible[Int32]`,
// so `Int16(from: Int32)` is rejected at type-check (the original
// `Int8(from: Int8)` case used to pass and then fail post-mono verification;
// since lossless conversions, every type converts from itself).
module Test

import std.numeric.(Int16, Int32, Int64)

protocol Sink[In] {
    func take(x: In) -> Int64
}

struct S: Sink[Int64], Sink[Bool] {
    func take(x: Int64) -> Int64 { x }
    func take(x: Bool) -> Int64 { 1 }
}

func f(s: S, t: String) -> Int64 {
    s.take(t) // ERROR(E107)
}

func g() {
    let a: Int32 = 5;
    let b = Int16(from: a); // ERROR(E107)
}
