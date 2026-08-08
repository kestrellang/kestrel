// test: execution
// stdlib: true
// expect-exit: 0

// docs/design/shared-box.md: "the aggregate copy fold already routes
// struct/enum copies through [`clone`] — a struct holding a handle becomes
// Cloneable and shares on copy for free". Copying `Holder` must therefore share
// its box (never bit-copy the handle, which would skip the share operation):
// the copies stay `isIdentical`, one mutation is seen by both, and a separately
// built Holder is a different storage.
module Test

import std.memory.(RcBox)
import std.numeric.(Int64)

// No hand-written `clone` and no explicit conformance — the fold supplies it.
struct Holder {
    var box: RcBox[Int64]
}

@main
func main() -> lang.i64 {
    let h1 = Holder(box: RcBox[Int64](1));
    let h2 = h1;                                          // aggregate copy -> RcBox.clone()
    let other = Holder(box: RcBox[Int64](1));             // equal payload, own storage

    if h1.box.isIdentical(to: h2.box) == false { return 1; }
    if h1.box.isIdentical(to: other.box) { return 2; }

    h2.box.sharedMutRef() = 99;                           // mutate through the copy
    if h1.box.getValue() != 99 { return 3; }              // one storage behind two holders
    if other.box.getValue() != 1 { return 4; }            // the independent box is untouched
    0
}
