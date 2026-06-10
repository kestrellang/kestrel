// test: diagnostics
// stdlib: false

// Arm-value decay of a NotCopyable pointee: the arm's copy-out reads the
// borrowed place, and a `not Copyable` value cannot be read out — the
// move-out-of-borrow guard fires (E503, the MIR-lowering backstop), on
// the ARM expression's span. Same family as binding decay
// (copy_out_notcopyable_rejected.ks).
module Test

struct Res: not Copyable {
    var v: lang.i64
}

struct Box {
    var r: Res
    func peek() -> &Res { self.r }
}

func pick(b: Box, c: lang.i1) -> Res {
    if c {
        b.peek() // ERROR(E503)
    } else {
        Res(v: 0)
    }
}
