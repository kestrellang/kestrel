// test: execution
// stdlib: true
// expect-exit: 0

// docs/design/shared-box.md: `protocol SharedBox: Cloneable, MutableIndirection`
// states the shared-ownership contract the compiler codes against, and `RcBox`
// is the `@lang(sharedBox)` default that must satisfy it. Every operation in
// `roundTrip` comes from the bound — init(consuming:), clone, pointeeRef,
// isUnique, isIdentical — never from RcBox's inherent API.
module Test

import std.memory.(RcBox, SharedBox)
import std.numeric.(Int64)
import std.core.(Bool)

// Generic over ANY conforming box; the body may only use SharedBox requirements.
func roundTrip[B](value: Int64) -> Int64 where B: SharedBox, B.Target = Int64 {
    let box: B = B(value);                            // init(consuming value: Target)
    if box.isUnique() == false { return -1; }         // a fresh box is the sole owner
    let alias = box.clone();                          // inherited Cloneable requirement
    if box.isIdentical(to: alias) == false { return -2; }
    if box.isUnique() { return -3; }                  // two handles now share storage
    let seen: Int64 = alias.pointeeRef();             // inherited Indirection read
    seen
}

@main
func main() -> lang.i64 {
    if roundTrip[RcBox[Int64]](7) != 7 { return 1; }
    0
}
