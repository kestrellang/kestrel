// test: execution
// stdlib: false

// A NORMAL closure captures by VIEW: `{ x }` holds a live reference to `x`, so
// a later `x = 20` is visible to the closure and `f()` returns 20 — behavior
// change #1 in docs/design/closures.md ("Behavior Changes from Today"). The
// snapshot semantics this file used to pin are now the OWNING tiers'
// (`escaping`/`consuming`), which are the returnable ones; see
// memory_model/closure_kinds/escaping/escaping_snapshot_at_creation.ks.
//
// Still verified by CALLING the closure locally — NOT by returning it: a view
// closure's environment points into this frame, so returning it dangles and is
// rejected (E494, see closure_return_with_capture).
module Main

@main
func test() -> lang.i64 {
    var x = 10;
    let f = { x };
    x = 20;
    // f() must be 20 (the live view), not the old 10 → i64_sub yields 0 (pass).
    lang.i64_sub(f(), 20)
}
