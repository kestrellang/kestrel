// test: execution
// stdlib: true
// expect-exit: 0

// A match that only destructures structs/tuples moves just the bound
// non-Copyable parts; the rest of the scrutinee stays usable and drops once,
// including when only one arm moves the part (partial-moves.md rule 7).
module Test

import std.numeric.(Int64)

var drops: Int64 = 0;

struct R: not Copyable {
    var id: Int64
    deinit { drops = drops + self.id; }
}
struct P: not Copyable { var a: R; var b: R; var tag: Int64 }

func take(consuming r: R) -> Int64 { r.id }

func matchPart(c: Bool) -> Int64 {
    let p = P(a: R(id: 1), b: R(id: 10), tag: 3);
    let n = match p {
        P { a: x, b: _, tag: 3 } => take(x),
        P { a: _, b: _, tag: _ } => 0
    };
    n + p.b.id
}

func tuplePart() -> Int64 {
    let t = (R(id: 1), R(id: 10));
    let n = match t {
        (x, _) => take(x)
    };
    n + t.1.id
}

@main
func main() -> Int64 {
    drops = 0;
    if matchPart(true) != 11 { return 1 }
    if drops != 11 { return 2 }
    drops = 0;
    if tuplePart() != 11 { return 3 }
    if drops != 11 { return 4 }
    0
}
