// test: execution
// stdlib: true
// backends: cranelift,llvm

// Regression (#196): `opt ?? refReturningCall()` — the `??` RHS desugars to a
// `() -> T` thunk, a value context, so the `&Int64` from `h.peek()` must decay
// to `Int64`. Before the fix the ref-misuse check (E491) fired first and the
// type mismatch was inverted; the closure-tail decay fix (#195) covers the
// thunk body uniformly.
module Test

struct Holder {
    var v: Int64
    func peek() -> &Int64 { self.v }
}

@main
func main() -> lang.i64 {
    let nothing: Int64? = .None;
    let h = Holder(v: 11);
    if (nothing ?? h.peek()) != 11 { return 1; }

    let present: Int64? = .Some(5);
    if (present ?? h.peek()) != 5 { return 2; }
    0
}
