// test: execution
// stdlib: true
// backends: cranelift,llvm
// expect-exit: 0
//
// Companion to #163: a non-Copyable value consumed in a loop body but
// reassigned before the back edge is valid — the back-edge re-use detection
// must NOT flag it (the seeded move is cleared by the reassignment). Each
// element is consumed exactly once per iteration; no double-free.

module Test

import std.numeric.Int64

public var drops: Int64 = 0;

struct Res: not Copyable {
    var id: Int64
    deinit { drops = drops + 1; }
}

func consume(consuming r: Res) {}

@main
func main() -> lang.i64 {
    var r = Res(id: 0);
    var i: Int64 = 0;
    while i < 3 {
        consume(r);       // moved...
        r = Res(id: i);   // ...then reinitialized before the back edge
        i = i + 1;
    }
    consume(r);           // final value consumed
    // 3 in-loop + 1 final = 4 consumes, each deinits once
    if drops != 4 { return 1 }
    return 0;
}
