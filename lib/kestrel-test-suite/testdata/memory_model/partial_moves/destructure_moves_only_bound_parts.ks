// test: execution
// stdlib: true
// expect-exit: 0

// `let` destructuring of a place, shorthand struct fields, and a tuple suffix
// after `..` move only the bound non-Copyable parts; the rest stays usable
// and every field drops once (partial-moves.md rule 7).
module Test

import std.numeric.(Int64)

var drops: Int64 = 0;

struct R: not Copyable {
    var id: Int64
    deinit { drops = drops + self.id; }
}
struct P: not Copyable { var a: R; var b: R }

func take(consuming r: R) -> Int64 { r.id }

func letTuple() -> Int64 {
    let t = (R(id: 1), R(id: 10));
    let (x, _) = t;
    take(x) + t.1.id
}

func letStructShorthand() -> Int64 {
    let p = P(a: R(id: 1), b: R(id: 10));
    let P { a, b: _ } = p;
    take(a) + p.b.id
}

func matchSuffix() -> Int64 {
    let t = (R(id: 1), R(id: 10), R(id: 100));
    let n = match t {
        (.., z) => take(z)
    };
    n + t.0.id + t.1.id
}

@main
func main() -> Int64 {
    drops = 0; if letTuple() != 11 { return 1 }            if drops != 11 { return 2 }
    drops = 0; if letStructShorthand() != 11 { return 3 }  if drops != 11 { return 4 }
    drops = 0; if matchSuffix() != 111 { return 5 }        if drops != 111 { return 6 }
    0
}
