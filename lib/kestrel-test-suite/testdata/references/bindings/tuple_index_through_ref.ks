// test: execution
// stdlib: true
// expect-exit: 0

// A tuple index reads through a reference like a field access does:
// `r.0` on `r: &(A, B)` is the pointee's element.

module Test

@main
func main() -> lang.i64 {
    let p = (1, 2);
    let r = &p;
    if r.0 != 1 { return 1 }
    if r.1 != 2 { return 2 }
    0
}
