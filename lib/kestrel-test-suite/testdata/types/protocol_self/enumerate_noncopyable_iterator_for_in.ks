// test: execution
// backends: cranelift,llvm
// expect-exit: 0

// `EnumerateIterator` is not Copyable, so `for` over it must hand the
// iterator to `Iterable.iter()` by value (`consuming`) rather than copy it.
// Before abstract `Self` stopped defaulting to Copyable, the blanket
// `extend Iterator: Iterable { func iter() -> Self { self } }` copied it and
// failed post-mono verification.

module Test

@main
func main() -> lang.i64 {
    let words: [String] = ["a", "bb", "ccc"];
    var total = 0;
    for (i, w) in words.iter().enumerate() {
        total = total + i * w.bytes.count;
    }
    let nums = [5, 6, 7];
    for (i, n) in nums.iter().enumerate() {
        total = total + i + n;
    }
    // words: 0*1 + 1*2 + 2*3 = 8; nums: (0+5) + (1+6) + (2+7) = 21
    if total != 29 { return 1 }
    0
}
