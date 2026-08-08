// test: execution
// stdlib: true
// expect-exit: 0

// Pins the kind prefix in *protocol requirement* signatures and in the witness
// that satisfies them — open question 3 of docs/design/closures.md ("kinds on
// function types in witness and generic contexts"). The requirement's kind and
// parameter convention must match the witness's exactly, and calls through a
// concrete conformer behave like the direct-call case.
module Test

import std.numeric.(Int64)

protocol Runner {
    func runOnce(consuming f: consuming () -> Int64) -> Int64
    func runEach(mutating f: mutating (Int64) -> ())
    func store(f: escaping () -> Int64) -> Int64
}

struct Simple: Runner {
    let tag: Int64

    func runOnce(consuming f: consuming () -> Int64) -> Int64 { f() }

    func runEach(mutating f: mutating (Int64) -> ()) { f(1); f(2); }

    func store(f: escaping () -> Int64) -> Int64 {
        let held = f;
        held()
    }
}

@main
func main() -> lang.i64 {
    let s = Simple(tag: 0);

    let a = 4;
    if s.runOnce({ a }) != 4 { return 1 }

    var total = 0;
    s.runEach({ total = total + it; });
    if total != 3 { return 2 }

    let b = 8;
    if s.store({ b }) != 8 { return 3 }

    0
}
