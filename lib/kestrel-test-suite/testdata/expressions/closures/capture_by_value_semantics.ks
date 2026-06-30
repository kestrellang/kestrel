// test: execution
// stdlib: false

// A closure captures by value: `{ x }` snapshots `x` (10) at creation, so a
// later `x = 20` does not change what the closure returns. Verified by CALLING
// the closure locally — NOT by returning it: a capturing closure's environment
// is stack-allocated in this frame, so returning it would dangle and is
// rejected (E494, see closure_return_with_capture). When heap-owned envs land,
// a by-value-capturing closure will become returnable; this test pins the
// snapshot semantics independently of that.
module Main

@main
func test() -> lang.i64 {
    var x = 10;
    let f = { x };
    x = 20;
    // f() must still be 10 (the snapshot), not 20 → i64_sub yields 0 (pass).
    lang.i64_sub(f(), 10)
}
