// test: execution
// stdlib: true
// backends: cranelift,llvm
// expect-exit: 0

// Regression (#143): writing a tuple element through a `mutating` parameter
// (`t.0 = ...` where `t` is a mutating param) shared the #198 no-op root
// cause — the FieldAddr projection now reaches the caller's storage through
// the mutable-receiver place, so the mutation is visible after the call.

module Test

import std.num.Int64

func bump(mutating t: (Int64, Int64)) {
    t.0 = t.0 + 10;
    t.1 = t.1 + 1;
}

@main
func main() -> Int64 {
    var u = (7, 8);
    bump(u);
    if u.0 != 17 { return 1 };
    if u.1 != 9 { return 2 };
    0
}
