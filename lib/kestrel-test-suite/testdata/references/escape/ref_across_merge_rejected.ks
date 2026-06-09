// test: diagnostics
// stdlib: false
// skip: stage1 — needs ref returns (S1) + MIR diagnostics in harness (T3)

// E-REF-15: a reference used as a PLACE cannot stay live across a block
// merge — here the ref-typed first argument is held open while the
// if-expression second argument splits the block. Deliberate v1 limit;
// the fix is hoisting the control-flow sibling into a binding first.
module Test

struct Holder {
    var v: lang.i64
    func peek() -> &lang.i64 { self.v }
}

// (body avoids `+`: lang.i64 has no Addable without the stdlib)
func add(a: lang.i64, b: lang.i64) -> lang.i64 { a }

func use(h: Holder, c: lang.i1) -> lang.i64 {
    add(h.peek(), if c { 1 } else { 2 }) // ERROR(E497)
}
