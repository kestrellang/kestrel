// test: execution
// stdlib: true
// expect-exit: 0
//
// #202 (BUG-68): the never-typed-divergence rule is not stdlib-specific — a
// user function declared `-> !` also diverges, so a `guard ... else { boom() }`
// must compile (no E003).

module Test

func boom() -> ! {
    fatalError("boom");
}

func positive(x: Int64) -> Int64 {
    guard x > 0 else { boom(); }
    return x + 5;
}

@main
func main() -> lang.i32 {
    if positive(3) != 8 { return 1 }
    0
}
