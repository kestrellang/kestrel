// test: execution
// stdlib: true

// Regression for the issue #121 follow-up: `match` on a `Bool` with a wildcard
// arm goes through the discriminant+switch path. `Bool` is a struct (newtype
// over `lang.i1`), not an enum, so codegen's discriminant width defaulted to
// I32 and a @guaranteed `Discriminant` load over-read a 1-byte Bool as 4 bytes
// of adjacent garbage — the tag compare against `true` never matched and the
// match always took the default (`k(true)` wrongly returned 2). The fix loads
// the scalar at its own repr width.
module Test

func k(b: Bool) -> Int64 {
    match b {
        true => 1,
        _ => 2
    }
}

// Mixed bool/tuple tree exercising both the boolean-branch and the
// discriminant+switch-with-default paths over Bool elements.
func three(a: Bool, b: Bool, c: Bool) -> Int64 {
    match (a, b, c) {
        (true, true, true) => 7,
        (true, true, false) => 6,
        (false, _, _) => 0,
        (_, _, _) => 9
    }
}

@main
func main() -> lang.i64 {
    if not (k(true) == 1) { return 1; }
    if not (k(false) == 2) { return 2; }
    if not (three(true, true, true) == 7) { return 3; }
    if not (three(true, true, false) == 6) { return 4; }
    if not (three(true, false, true) == 9) { return 5; }
    if not (three(false, true, true) == 0) { return 6; }
    return 0;
}
