// test: diagnostics
// stdlib: true

// Stage 2d: a protocol-dispatched OPERATOR on a ref-instantiated container
// rejects CLEANLY. `Optional[&Int64]`'s Equal arrives via the blanket
// `extend Equatable: Equal[Self]`; the genuine requirement — the receiver
// satisfies Equatable, i.e. `extend Optional[T]: Equatable where T:
// Equatable` holds at T = &Int64 — fails the ref gate (a ref type arg
// satisfies only Copyable until refs get real witnesses). This used to
// bypass the gate entirely and ICE post-mono ("Callee::Witness not
// resolved").
module Test

@main
func main() {
    var x = 1;
    let r = &x;
    let a: Optional[&Int64] = .Some(r);
    let b: Optional[&Int64] = .None;
    let eq = a == b; // ERROR: !: Equal
    let lt = a < b; // ERROR: !: Less
}
