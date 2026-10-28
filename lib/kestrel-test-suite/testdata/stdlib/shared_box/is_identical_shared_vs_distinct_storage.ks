// test: execution
// stdlib: true
// expect-exit: 0

// docs/design/shared-box.md: `isIdentical(to:)` answers "same managed storage?"
// — it backs class identity (`===`) and is required instead of exposing an
// address. Handles derived from one `init` (by `clone` or by the copy fold) are
// identical; handles from independent `init`s are not, even with equal payloads.
module Test

import std.memory.(RcBox)
import std.numeric.(Int64)

@main
func main() -> lang.i64 {
    let a = RcBox[Int64](1);
    let b = a.clone();              // explicit share
    let c = a;                      // implicit share via the copy fold
    let other = RcBox[Int64](1);    // equal payload, independent storage

    if a.isIdentical(to: a) == false { return 1; }      // reflexive
    if a.isIdentical(to: b) == false { return 2; }
    if a.isIdentical(to: c) == false { return 3; }
    if b.isIdentical(to: c) == false { return 4; }

    if a.isIdentical(to: other) { return 5; }           // value equality is not identity
    if b.isIdentical(to: other) { return 6; }
    0
}
