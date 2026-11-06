// test: diagnostics
// stdlib: true

// Fuzz H20: a call resolved through a generic protocol's member must check
// the protocol's ARGS, not just the protocol. `Int8` conforms to
// `Convertible[Int16]`, `[Int32]`, … but not `Convertible[Int8]`, so
// `Int8(from: Int8)` is rejected at type-check (it used to pass and then
// fail post-mono verification).
module Test

import std.numeric.(Int8, Int64)

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
    let a: Int8 = 5;
    let b = Int8(from: a); // ERROR(E107)
}
