// test: diagnostics
// stdlib: true

// References 2b: a SHARED (`&`) payload binding is read-only — writing
// through it is the shared-ref assignment rejection.
module Test

func f() {
    var x = 1;
    let r = &x;
    let o: Optional[&Int64] = .Some(r);
    if let .Some(v) = o {
        v = 5; // ERROR(E208)
    }
}
