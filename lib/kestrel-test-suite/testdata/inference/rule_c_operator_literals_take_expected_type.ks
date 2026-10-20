// test: execution
// stdlib: true
// expect-exit: 0

// bidi design §4 Rule C: an operator over literals is typed by the type its
// result is expected to have. Candidates are the types the expectation names
// plus the literal default; the one whose operator returns the expected type
// wins. `1 == 2` against `Bool` keeps the default (`Int64`): its `Output` is
// `Bool`, and `Bool` itself is not an integer literal type.

module Test

func takes8(v: Int8) -> Int8 { v }

@main
func main() -> lang.i64 {
    let y: Int8 = 100 + 27;
    if y != 127 { return 1; }
    let n: Int8 = -5;
    if n != -5 { return 2; }
    let c: Bool = 1 == 2;
    if c { return 3; }
    if takes8(-1) != -1 { return 4; }
    let r: ClosedRange[Int16] = 100..=200;
    if r.start != 100 { return 5; }
    0
}
