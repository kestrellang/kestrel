// test: execution
// stdlib: false
// expect-exit: 0

// The flagship of docs/design/closures.md, "Capture Rules → How it is captured":
// a normal-kind closure captures a VIEW of the frame, so `{ x }` reads `x`
// through a live reference. A later `x = 20` is therefore visible to `f()`,
// which returns 20 — not the 10 the interim snapshot model produced.
module Test

@main
func main() -> lang.i64 {
    var x = 10;
    let f = { x };   // live view of `x`, not a snapshot
    x = 20;
    // f() must be 20 under view capture → i64_sub yields 0 (pass).
    lang.i64_sub(f(), 20)
}
