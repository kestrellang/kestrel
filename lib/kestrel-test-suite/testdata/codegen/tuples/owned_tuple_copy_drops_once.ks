// test: execution
// stdlib: true

// Regression: copying an @owned tuple that owns a heap (resource) member must
// deep-clone it, not bitwise-alias it.
//
// `pair` below is an @owned `(String, Int64)` unwrapped from the Optional and
// kept live — its `String` field is only *borrowed* (`pair.0.isEmpty`), so the
// whole tuple is dropped at scope exit. Lowering copies `pair` (the original
// stays live for that drop); the mono-expand `CopyValue` tuple arm used to
// alias an @owned tuple copy instead of cloning it, so the copy and the
// original dropped the same `String` heap → double-free → crash. The copy side
// must mirror the destroy side and deep-clone tuple members. This is the
// minimized form of the bootstrapped-`flock` crash (TOML `nextLine` returns
// `Optional[(String, Int64)]`).
module Test

func makeLine(n: Int64) -> Optional[(String, Int64)] {
    var s = String();
    s.append("heap-allocated-line-content-");
    s.append("xxxxxxxxxxxxxxxxxxxxxxxxxxxx");
    .Some((s, n))
}

@main
func main() -> lang.i64 {
    var i: Int64 = 0;
    var seen: Int64 = 0;
    while i < 64 {
        if let .Some(pair) = makeLine(i) {
            // Borrow a field (does not move it out) so `pair` keeps the tuple
            // and is dropped whole at scope exit — the double-free path.
            if pair.0.isEmpty == false {
                seen = seen + pair.1
            }
        };
        i = i + 1
    };
    // sum 0..<64 == 2016; a wrong/garbage read or a crash fails the test.
    if seen == 2016 { return 0 };
    1
}
