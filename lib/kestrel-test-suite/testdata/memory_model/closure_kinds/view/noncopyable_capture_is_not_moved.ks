// test: execution
// stdlib: false
// expect-exit: 0

// docs/design/closures.md, "Behavior Changes from Today" #3: "Capturing a
// non-Copyable value no longer kills the original in view kinds — it is merely
// frozen against destruction." Using `r` after building the view closure is
// therefore legal; today it is E500 (see
// expressions/closures/use_after_capture_noncopyable.ks).
module Test

struct Res: not Copyable {
    var v: lang.i64
}

@main
func main() -> lang.i64 {
    let r = Res(v: 42);
    let f = { r.v };                            // view — no move of `r`
    if lang.i64_eq(f(), 42) { } else { return 1; }
    if lang.i64_eq(r.v, 42) { } else { return 2; }   // `r` still live: no use-after-move
    if lang.i64_eq(f(), 42) { } else { return 3; }
    0
}
