// test: execution
// stdlib: true
// expect-exit: 0

module Test

// Regression: implicit-`it` detection walks the closure CST, where an
// interpolated string is a single raw token (holes are only re-parsed during
// body lowering). An `it` referenced only inside a `\(...)` hole was invisible
// to the walk, so the closure was never marked as an it-closure and name-res
// failed with "undefined name 'it'". Holes are now extracted and re-parsed
// during detection.

func applyStr(f: (Int64) -> String) -> String {
    f(7)
}

func callThunk(f: () -> String) -> String {
    f()
}

func describe(s: String) -> String {
    s
}

@main
func main() -> lang.i64 {
    // `it` referenced ONLY inside an interpolation hole
    if applyStr({ "got \(it)" }) != "got 7" { return 1 }

    // multiple holes, including one mixing `it` with an expression
    if applyStr({ "n=\(it) twice=\(it + it)" }) != "n=7 twice=14" { return 2 }

    // `it` in a hole that carries a format spec
    if applyStr({ "hex=\(it:x)" }) != "hex=7" { return 3 }

    // `it` in a multiline-string hole
    let m = applyStr({ """
      multi \(it)
      """ });
    if m != "multi 7" { return 4 }

    // nested it-closure inside a hole: the outer closure must stay
    // zero-param — the inner closure's `it` is its own
    let xs = [1, 2, 3];
    if callThunk({ "mapped \(xs.map { it * 2 }.count)" }) != "mapped 3" { return 5 }

    // `it` inside a nested string literal inside a hole
    if applyStr({ "outer \(describe("inner \(it)"))" }) != "outer inner 7" { return 6 }

    // escaped `\\(` is NOT a hole — the closure must stay zero-param and
    // print the text literally
    if callThunk({ "not a hole \\(it)" }) != "not a hole \\(it)" { return 7 }

    0
}
