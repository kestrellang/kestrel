// test: execution
// stdlib: true
// expect-exit: 0

// #144: a failable init delegating to a FAILING failable init must propagate
// `.None` and must NOT double-drop. The inner init partially initializes self
// (self.a = Res) then fails; it unwinds its own field (one deinit). The outer
// init must then return `.None` — not unconditionally `.Some(self)` with a
// stale, already-dropped `self.a` (which previously caused a second deinit).
//
// Buggy behavior was: ".Some" taken, `v.a.id` read as 11 (stale), and `d`
// reaching 2 (double-drop). Correct: `.None`, `d == 1`.

module Test

import std.numeric.Int64

public var dropCount: Int64 = 0;

struct Res: not Copyable {
    var id: Int64
    deinit { dropCount = dropCount + 1; }
}

struct Chain: not Copyable {
    var a: Res
    init(inner i: Int64)? {
        self.a = Res(id: 11);
        if i == 1 { return null; }
    }
    init(outer o: Int64)? { self.init(inner: o); }
}

@main
func main() -> lang.i32 {
    // Delegate to a FAILING inner init → outer must be .None, exactly one drop.
    match Chain(outer: 1) {
        .None => {},
        .Some(v) => { return 10 }   // stale-read bug would land here
    }
    if dropCount != 1 { return 20 } // double-drop bug would make this 2

    // Delegate to a SUCCEEDING inner init → outer is .Some, value usable, and
    // the single live Res drops exactly once at scope exit.
    match Chain(outer: 0) {
        .None => { return 30 },
        .Some(v) => { if v.a.id != 11 { return 31 } }
    }
    if dropCount != 2 { return 40 }
    0
}
