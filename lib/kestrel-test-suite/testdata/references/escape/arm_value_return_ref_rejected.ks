// test: diagnostics
// stdlib: false

// A match/if in RETURN position of a ref-returning function is STILL an
// error after arm-value decay — arms always produce owned values, so the
// implicit return borrow roots at the merge result (a local), and the
// escape checker rejects it (E494). Before stage 1.5 this shape died as
// E497 (ref across merge); the decay shifts it to the better diagnostic.
module Test

struct Box {
    var v: lang.i64
    func peek() -> &lang.i64 { self.v }
    // A method: the receiver is the unambiguous root (no E493).
    func pick(c: lang.i1) -> &lang.i64 {
        if c { self.peek() } else { self.peek() } // ERROR(E494)
    }
}
