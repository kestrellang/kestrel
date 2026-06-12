// test: execution
// stdlib: true
// backends: cranelift,llvm

// Protocol-dispatched OPERATORS on a ref-instantiated container WORK:
// `Optional[&Int64]`'s Equal arrives via the blanket `extend Equatable:
// Equal[Self]`, whose requirement — `extend Optional[T]: Equatable where
// T: Equatable` holds at T = &Int64 — is satisfied by the stdlib
// forwarding extension `extend &T: Equatable where T: Equatable`
// (core/ref.ks). Comparison rides `extend &T: Comparable` the same way.
// History: this used to ICE post-mono ("Callee::Witness not resolved"),
// then cleanly reject (the 2d gate); ref extensions make it dispatch the
// POINTEE's witnesses.
module Test

@main
func main() -> Int64 {
    var x = 1;
    var y = 1;
    var z = 2;
    let rx = &x;
    let ry = &y;
    let rz = &z;
    let a: Optional[&Int64] = .Some(rx);
    let b: Optional[&Int64] = .Some(ry);
    let c: Optional[&Int64] = .Some(rz);
    let n: Optional[&Int64] = .None;

    if not (a == b) { return 1; }
    if a == c { return 2; }
    if a == n { return 3; }
    if not (n == n) { return 4; }
    if a != b { return 5; }
    if not (a < c) { return 6; }
    if c < a { return 7; }

    // Bare refs: `==` on ref OPERANDS peels at the receiver (stage 1) —
    // unchanged by the extensions, pinned here against regressions.
    if not (rx == ry) { return 8; }
    if rx == rz { return 9; }
    0
}
