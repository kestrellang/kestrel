// test: execution
// stdlib: false
// expect-exit: 0

// docs/design/closures.md, "The freeze rule": the freeze is LEXICAL — it runs
// to the end of the extent of every binding that may carry the view. Once the
// inner block holding `f` ends, nothing carries a view of `r`, so consuming `r`
// is legal again. This is the positive counterpart of the E507 tests.
module Test

struct Res: not Copyable {
    var v: lang.i64
}

func sink(consuming r: Res) -> lang.i64 { r.v }

@main
func main() -> lang.i64 {
    let r = Res(v: 7);
    if lang.i64_eq(1, 1) {
        let f = { r.v };    // freeze starts here, ends with this block
        if lang.i64_eq(f(), 7) { } else { return 1; }
    }
    // no live view of `r` any more — destruction is allowed
    if lang.i64_eq(sink(r), 7) { } else { return 2; }
    0
}
