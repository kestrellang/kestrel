// test: diagnostics
// stdlib: true

// A member provided only by specialized extensions that DON'T apply to the
// concrete receiver must be a clean "no member" error — not silently resolved
// (and then ICE'd at monomorphization on the missing witness). Here `Show` is
// provided for `Box` only via `extend Box[lang.i64]` / `extend Box[lang.i32]`,
// but the receiver is `Box[Int64]` (the Int64 STRUCT, distinct from the raw
// `lang.i64` primitive), so no extension applies.

module Test

import std.numeric.Int64

protocol Show { func show() -> lang.i64 }

struct Box[T] { var value: T }

extend Box[lang.i64]: Show { func show() -> lang.i64 { 1 } }
extend Box[lang.i32]: Show { func show() -> lang.i64 { 2 } }

func test() -> lang.i64 {
    let b: Box[Int64] = Box(value: 5);
    b.show() // ERROR: no member 'show' on type 'Box[Int64]'
}
