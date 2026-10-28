// test: execution
// stdlib: true
// expect-exit: 0
//
// #202 (BUG-68): a `guard ... else { ... }` whose else block ends in a
// never-typed call (`fatalError`, type `!`) diverges, so E003 must NOT fire.
// The divergence check used to be a syntactic whitelist (return/break/
// continue/throw) and ignored the trailing expression's `!` type.

module Test

func positive(x: Int64) -> Int64 {
    guard x > 0 else { fatalError("negative"); }
    return x + 5;
}

@main
func main() -> lang.i32 {
    if positive(2) != 7 { return 1 }
    0
}
