// test: execution
// stdlib: false
// backends: cranelift,llvm

// Named-ref-binding TYPING smoke (stage 1.5 item 2, infer half):
// `let r = &x` types as `&i64` (no binding decay); reading the binding in
// a value context decays to a copy (`let s = r` → i64); `&r` re-borrows.
// MIR aliasing semantics (write-through visibility, store-through) are
// pinned separately once the place lowering lands.
module Test

@main
func main() -> lang.i64 {
    var x: lang.i64 = 41;
    let r = &x;
    let s = r;
    let t = &r;
    let u = t;
    let m = &mutating x;
    let w = m;
    if lang.i64_eq(s, 41) { } else { return 1; }
    if lang.i64_eq(u, 41) { } else { return 2; }
    if lang.i64_eq(w, 41) { } else { return 3; }
    0
}
