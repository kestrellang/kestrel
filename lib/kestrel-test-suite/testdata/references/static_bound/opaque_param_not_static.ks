// test: execution
// stdlib: true
// expect-exit: 0

// `some P and not Static` in parameter position relaxes the implicit
// `Static` bound, like `where T: not Static` on a named generic: a
// borrowing `ArraySlice` is accepted, and a defaulted argument still
// type-checks (the default's annotation used to reject `not Static`, E140).

module Test

import std.collections.(Array, ArraySlice, Sliceable)

func total(xs: some Sliceable[Int64] and not Static) -> Int64 {
    var t: Int64 = 0;
    for x in xs.asUnsafeSlice() { t += x; }
    t
}

func totalOr(xs: some Sliceable[Int64] and not Static = Array[Int64]()) -> Int64 {
    total(xs)
}

@main
func main() -> lang.i64 {
    let xs = [1, 2, 3, 4];
    if total(xs) != 10 { return 1 }
    if total(xs(1..)) != 9 { return 2 }      // borrowing slice: not Static
    if totalOr(xs(..<2)) != 3 { return 3 }
    if totalOr() != 0 { return 4 }          // default argument
    0
}
