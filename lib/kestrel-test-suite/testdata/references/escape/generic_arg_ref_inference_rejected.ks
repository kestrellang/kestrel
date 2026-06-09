// test: diagnostics
// stdlib: true
// skip: stage1 — needs stage-1 front-end (S1)

// E-REF-19: a ref must never leak into a generic type argument through
// INFERENCE (annotations are already rejected by the stage-0.5 walk).
// Array literal elements unify by equality, not coercion, so `[h.peek()]`
// would infer `Array[&Int64]` — the validation phase rejects it with a
// bind-the-value-first hint. (Deliberate stage-1 gap; element decay is 1.5.)
module Test

struct Holder {
    var v: Int64
    func peek() -> &Int64 { self.v }
}

func use(h: Holder) {
    let xs = [h.peek()]; // ERROR(E492)
}
