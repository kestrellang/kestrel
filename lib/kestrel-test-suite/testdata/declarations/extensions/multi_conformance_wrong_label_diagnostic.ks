// test: diagnostics
// stdlib: true

// When a type conforms to the SAME parameterized protocol more than once
// (`Conv[Int64]` + `Conv[Int32]`), calling its requirement with a WRONG argument
// label must still produce the precise "wrong argument label" diagnostic — not a
// vague "no member". `conv(x: In)` is a single-name (positional) param, so the
// `x:` label at the call site is invalid.
//
// Before the fix, member resolution couldn't recover a single candidate from the
// two conformances (the `0 label-matches` arm only handled the lone-candidate
// case) and degraded to "no member 'conv'". The two `conv` impls share one label
// signature, so a representative is recovered for the label diagnostic.
module Main

protocol Conv[In] { func conv(x: In) -> Int64 }

struct S { var x: Int64; }

extend S: Conv[Int64] { func conv(x: Int64) -> Int64 { 10 } }
extend S: Conv[Int32] { func conv(x: Int32) -> Int64 { 20 } }

func test(s: S) -> Int64 {
    s.conv(x: 5) // ERROR: wrong argument label
}
