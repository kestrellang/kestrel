// test: diagnostics
// stdlib: true

// References 2b owned-return escape: a ref-bearing aggregate tainted by a
// LOCAL cannot leave the frame (the carrier variant of E494).
module Test

func dangle() -> Optional[&Int64] {
    var x = 1;
    let r = &x;
    let o: Optional[&Int64] = .Some(r);
    o // ERROR(E494)
}
