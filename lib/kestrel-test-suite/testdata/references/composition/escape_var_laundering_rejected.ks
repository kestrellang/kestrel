// test: diagnostics
// stdlib: true

// References 2b (the G1 closure): storing a LOCAL-tainted ref-bearing
// value into a var slot taints the SLOT (monotone join); loads inherit —
// the memory roundtrip cannot launder the escape taint.
module Test

func launder() -> Optional[&Int64] {
    var x = 1;
    let r = &x;
    var o: Optional[&Int64] = .Some(r);
    o // ERROR(E494)
}
