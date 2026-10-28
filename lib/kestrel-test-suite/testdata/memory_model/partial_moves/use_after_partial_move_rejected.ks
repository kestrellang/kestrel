// test: diagnostics
// stdlib: true

// Partial moves: after `let x = p.a`, `p.a` and `p` as a whole are unusable
// (E500, "partially moved" for the whole); disjoint `p.b` stays usable. A
// field moved on one branch is maybe-moved (E501). Moving through a type with
// a deinit is E511; out of a borrowed param is E503.
module Test

import std.numeric.(Int64)

struct R: not Copyable { var id: Int64 }
struct P: not Copyable { var a: R; var b: R; func sum() -> Int64 { 0 } }
struct D: not Copyable { var a: R; deinit {} }
struct Q: not Copyable { var d: D }

func take(consuming r: R) -> Int64 { r.id }
func takeP(consuming p: P) -> Int64 { 0 }

func useMovedField() -> Int64 {
    let p = P(a: R(id: 1), b: R(id: 2));
    let x = p.a;
    take(p.a) // ERROR(E500)
}

func useWhole() -> Int64 {
    let p = P(a: R(id: 1), b: R(id: 2));
    let x = p.a;
    takeP(p) // ERROR(E500)
}

func methodOnPartial() -> Int64 {
    let p = P(a: R(id: 1), b: R(id: 2));
    let x = p.a;
    p.sum() // ERROR(E500)
}

func otherFieldOk() -> Int64 {
    let p = P(a: R(id: 1), b: R(id: 2));
    let x = p.a;
    take(p.b) + x.id
}

func maybeMoved(c: Bool) -> Int64 {
    let p = P(a: R(id: 1), b: R(id: 2));
    if c { take(p.a); }
    take(p.a) // ERROR(E501)
}

func throughDeinit(consuming q: Q) -> Int64 {
    take(q.d.a) // ERROR(E511)
}

func borrowedField(p: P) -> Int64 {
    take(p.a) // ERROR(E503)
}
