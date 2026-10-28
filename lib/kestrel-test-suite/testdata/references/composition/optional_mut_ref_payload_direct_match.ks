// test: execution
// backends: cranelift,llvm
// stdlib: true

// Store-through a `.Some(w)` binding whose scrutinee resolves LATE: the
// receiver's type waits on literal defaulting (Cell(v: 5) → T = ?int),
// so the deferred ImplicitPat delivers w's `&mutating` type after the
// arm body has generated. The assignment must not pin `w` from its RHS
// literal first (it used to: "expected Int64 got &mutating Int64") —
// AssignTarget defers until the target's type arrives, then routes the
// RHS at the POINTEE (store-through). Both the direct-match and the
// unannotated-local forms are pinned here; the annotated form already
// worked (mutating_ref_payload_store_through.ks).
module Test

struct Cell[T] {
    var v: T

    mutating func pick() -> Optional[&mutating T] {
        let r = &mutating self.v;
        let o: Optional[&mutating T] = .Some(r);
        o
    }
}

@main
func main() -> lang.i64 {
    // Direct match on the call result.
    var c = Cell(v: 5);
    if let .Some(w) = c.pick() {
        w = 99;
    }
    if c.v != 99 { return 1; }

    // Unannotated local between call and match.
    var d = Cell(v: 7);
    let m = d.pick();
    if let .Some(w2) = m {
        w2 = 42;
    }
    if d.v != 42 { return 2; }
    0
}
