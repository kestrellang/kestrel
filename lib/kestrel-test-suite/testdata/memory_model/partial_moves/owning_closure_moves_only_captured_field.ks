// test: execution
// stdlib: true
// expect-exit: 0

// An owning closure (`escaping`/`consuming`) whose captures of a local are all
// field reads moves just the non-Copyable field it reads; the local's other
// fields stay usable and drop exactly once (partial-moves.md, phase 4).
module Test

import std.numeric.(Int64)

var drops: Int64 = 0;

struct R: not Copyable {
    var id: Int64
    deinit { drops = drops + self.id; }
}
struct H: not Copyable { var r: R; var s: R; var n: Int64 }

func run(f: escaping () -> Int64) -> Int64 { f() }
func once(consuming f: consuming () -> Int64) -> Int64 { f() }

func escapingField() -> Int64 {
    let h = H(r: R(id: 1), s: R(id: 10), n: 5);
    let f: escaping () -> Int64 = { h.r.id + h.n };
    let a = run(f);
    a + h.s.id
}

func consumingField() -> Int64 {
    let h = H(r: R(id: 1), s: R(id: 10), n: 5);
    let n = once({ h.r.id });
    n + h.s.id
}

@main
func main() -> Int64 {
    drops = 0;
    if escapingField() != 16 { return 1 }
    if drops != 11 { return 2 }
    drops = 0;
    if consumingField() != 11 { return 3 }
    if drops != 11 { return 4 }
    0
}
