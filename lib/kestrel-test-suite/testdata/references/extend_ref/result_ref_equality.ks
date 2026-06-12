// test: execution
// stdlib: true
// backends: cranelift,llvm

// `Result[&Int64, String]: Equatable` — the conditional conformance
// (`where T: Equatable, E: Equatable`) holds at a REF success type via the
// stdlib `extend &T: Equatable` forwarding extension; `==` compares the
// pointed-at values.
module Test

@main
func main() -> Int64 {
    var a = 7;
    var b = 7;
    var c = 8;
    let ra = &a;
    let rb = &b;
    let rc = &c;

    let x: Result[&Int64, String] = .Ok(ra);
    let y: Result[&Int64, String] = .Ok(rb);
    let z: Result[&Int64, String] = .Ok(rc);
    let e: Result[&Int64, String] = .Err("nope");

    if not (x == y) { return 1; }   // Ok(7) == Ok(7) by pointee
    if x == z { return 2; }         // Ok(7) != Ok(8)
    if x == e { return 3; }         // Ok != Err
    if not (e == e) { return 4; }   // Err("nope") == Err("nope")
    0
}
