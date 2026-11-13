// test: execution
// stdlib: true
// expect-exit: 0

// A declared extension param must stay free when the target's own args are
// concrete. Today `U` in `extend[U] Box[Bool]` is bound to the target's
// first arg (`Bool`), so `tagAs(seven)` fails with "expected Bool got
// Int64". The same happened with the old implicit form.

module Test

import std.numeric.Int64

protocol TagAs[U] {
    func tagAs(u: U) -> U
}

struct Box[T] {
    var t: T
}

extend[U] Box[Bool]: TagAs[U] {
    func tagAs(u: U) -> U { u }
}

@main
func main() -> lang.i32 {
    let b = Box[Bool](t: true);
    let seven: Int64 = 7;
    if b.tagAs(seven) != 7 { return 10 }
    0
}
