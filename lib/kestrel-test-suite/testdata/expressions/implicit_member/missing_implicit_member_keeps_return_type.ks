// test: diagnostics
// stdlib: true

// A missing `.member` in a `return` must not poison the function's return type:
// the next bad `.member` is still reported, against the real type. Poisoning
// the shared var printed "not found on _" and silenced every later one.

module Test

func f(x: Int64) -> Int64? {
    if x > 0 { return .nope } // ERROR: implicit member '.nope' not found
    return .nada // ERROR: implicit member '.nada' not found
}
