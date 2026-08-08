// test: execution
// stdlib: true
// expect-exit: 0

// ============================================================================
// PINS A KNOWN v1 LIMITATION — NOT the desired end state.
//
// A `consuming` closure is ONE-SHOT: "the call consumes the value, and a
// second call is an ordinary use-after-move" (docs/design/closures.md, and
// plan D8's E500 extension site 1). That rule is enforced through the MOVE
// CHECKER, which records a move of the CALLEE LOCAL when the callee's resolved
// type is consuming-kind — see the sibling
// `second_call_of_consuming_closure_is_use_after_move.ks`, where a second
// `f()` on a `let`/param-held closure is a clean E500.
//
// The move checker is LocalId-granular: `rhs_local` deliberately returns
// `None` for any projection, because there is no partial-move model for
// fields. So when the one-shot value lives in a STRUCT FIELD, the call site
// `(h.f)()` has no bare local to mark moved, and calling it twice is accepted
// — as this test records. Each call takes the environment out of its unique
// box again and the caller releases the (already emptied) block again.
//
// FOLLOW-UP: give the move checker a field-granular move model (the same
// `PlaceKey` machinery the freeze rule already uses, plan D8), then flip this
// file to a `diagnostics` test asserting E500 on the second call.
//
// Kept capture-free ON PURPOSE: a capture-free owning closure carries a null
// environment handle and never allocates, so pinning today's behavior here
// records the missing DIAGNOSTIC without also pinning undefined behavior. The
// capturing form compiles today too, and is genuinely unsound.
// ============================================================================
module Test

import std.numeric.Int64

struct Holder: not Copyable {
    var f: consuming () -> Int64
}

@main
func main() -> lang.i64 {
    var h = Holder(f: { () in 5 });
    let a = (h.f)();
    let b = (h.f)();   // NOT rejected today — see the header
    if a != 5 { return 1 }
    if b != 5 { return 2 }
    0
}
