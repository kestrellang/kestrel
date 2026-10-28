// test: execution
// backends: cranelift,llvm
// stdlib: true

// A pattern BINDER's type comes from its pattern, never its uses — the
// USE-SITE half of the AssignTarget principle. The scrutinee here waits
// on literal defaulting (Cell(v: 5) → T = ?int), so the deferred
// ImplicitPat delivers w's `&mutating Int64` late; the loop-body uses
// (`w + 1`, an operator ARG coerce) must not pin `w` to Int64 first
// (they used to: plain unify → late payload equate → "expected Int64
// got &mutating Int64"). The pattern-binder gate defers those coerces
// until the pattern fires.
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
    var c = Cell(v: 5);
    if let .Some(w) = c.pick() {
        let s = w + 1;
        if s != 6 { return 1; }
    }

    // The shared-read sibling: a while-let loop whose binder feeds an
    // operator before the literal default lands.
    var d = Cell(v: 7);
    var sum = 0;
    if let .Some(w2) = d.pick() {
        sum = sum + w2;
    }
    if sum != 7 { return 2; }
    0
}
