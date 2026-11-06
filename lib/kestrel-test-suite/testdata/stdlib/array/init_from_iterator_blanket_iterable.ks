// test: execution
// stdlib: true
// expect-exit: 0

// Collecting an Iterator into an Array. Originally a regression (#132) for
// `Array(from: iterator)` through the blanket `extend Iterator: Iterable`,
// which ICEd in post-mono verify. That blanket conformance is gone (an
// iterator is no longer Iterable — `Iterable.iter()` borrows), so an iterator
// is collected with `collect()`.

module Test

@main
func main() -> lang.i64 {
    // Iterator via String bytes (the original #132 repro).
    let a = "salt".bytes.iter().collect();
    if a.count != 4 { return 1 }
    if a(0) != 115 { return 2 }  // 's'

    // Iterator obtained from an Array's own iterator.
    let nums = [10, 20, 30];
    let b = nums.iter().collect();
    if b.count != 3 { return 3 }
    if b(2) != 30 { return 4 }

    0
}
