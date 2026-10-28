// test: execution
// stdlib: true
// backends: cranelift,llvm

// A ref-returning CALL in bare return-tail position must decay to the
// pointee when the declared return type is non-ref — the fifth decay site
// (alongside let-init, scrutinee, assign-target, arm-value). Before the fix,
// `deref`'s tail typed as `&Int64` and failed to coerce to `Int64`
// (`expected Int64, got &Int64`); binding through a `let` first was the only
// workaround. `peek` (return type `&Int64`) must still pass the ref through.
module Test

struct Box {
    var v: Int64

    func peek() -> &Int64 { self.v }      // return-position borrow: keeps &T
    func deref() -> Int64 { self.peek() } // ref-call tail decays to the pointee
}

@main
func main() -> lang.i64 {
    let b = Box(v: 7);
    if b.deref() != 7 { return 1; }
    0
}
