// test: execution
// stdlib: false
// backends: cranelift,llvm

// `&mutating` binding store-through (ratified): `m = v` writes the
// REFERENT (there is no rebind spelling). RHS reads through the binding
// happen before the store (RHS-first order). Reads of the var while its
// mut borrow is live are the decided may-alias behavior.
module Test

@main
func main() -> lang.i64 {
    var x: lang.i64 = 1;
    let m = &mutating x;
    m = 42;
    let snap1 = x;
    m = lang.i64_add(m, 7);
    let snap2 = x;
    let last = m;
    if lang.i64_eq(snap1, 42) { } else { return 1; }
    if lang.i64_eq(snap2, 49) { } else { return 2; }
    if lang.i64_eq(last, 49) { } else { return 3; }
    0
}
