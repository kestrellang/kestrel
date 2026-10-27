// test: execution
// stdlib: true
// expect-exit: 0

// Partial moves (docs/design/partial-moves.md): a non-Copyable field moves
// out of an owned local; the other fields stay and drop individually. Each R
// adds its id to `drops`, so every case checks each field drops exactly once:
// let/arg/tuple/nested moves, a move on one branch (drop flag), field reinit,
// whole reassignment, and `consuming self` returning a field (fuzz M6).
module Test

import std.numeric.(Int64)

var drops: Int64 = 0;

struct R: not Copyable {
    var id: Int64
    deinit { drops = drops + self.id; }
}
struct P: not Copyable { var a: R; var b: R }
struct N: not Copyable { var p: P; var c: R }

func take(consuming r: R) -> Int64 { r.id }

// let x = p.a: p.b drops at scope end, p.a when x does.
func letMove() -> Int64 {
    let p = P(a: R(id: 1), b: R(id: 10));
    let x = p.a;
    x.id
}

// Consuming argument.
func argMove() -> Int64 {
    let p = P(a: R(id: 1), b: R(id: 10));
    take(p.a)
}

// Tuple element of a consuming param.
func tupleMove(consuming t: (R, R)) -> Int64 {
    let x = t.0;
    x.id
}

// Nested path.
func nestedMove() -> Int64 {
    let n = N(p: P(a: R(id: 1), b: R(id: 10)), c: R(id: 100));
    take(n.p.a)
}

// Moved on one branch only: the flag decides.
func branchMove(c: Bool) -> Int64 {
    let p = P(a: R(id: 1), b: R(id: 10));
    if c { return take(p.a) }
    0
}

func branchMove2(c: Bool) -> Int64 {
    let p = P(a: R(id: 1), b: R(id: 10));
    var n: Int64 = 0;
    if c { n = take(p.a); }
    n
}

// Reinit after move.
func reinit() -> Int64 {
    var p = P(a: R(id: 1), b: R(id: 10));
    let n = take(p.a);
    p.a = R(id: 1000);
    n
}

// Whole reassign of a partially moved var.
func wholeReassign() -> Int64 {
    var p = P(a: R(id: 1), b: R(id: 10));
    let n = take(p.a);
    p = P(a: R(id: 100), b: R(id: 1000));
    n
}

struct S: not Copyable {
    var inner: R
    var other: R
    consuming func into() -> R { self.inner }
}

func expect(label: Int64, got: Int64, want: Int64) -> Int64 {
    if got != want { return label }
    0
}

@main
func main() -> Int64 {
    drops = 0; letMove();          if drops != 11 { return 1 }
    drops = 0; argMove();          if drops != 11 { return 2 }
    drops = 0; tupleMove((R(id: 1), R(id: 10)));   if drops != 11 { return 3 }
    drops = 0; nestedMove();       if drops != 111 { return 4 }
    drops = 0; branchMove(true);   if drops != 11 { return 5 }
    drops = 0; branchMove(false);  if drops != 11 { return 6 }
    drops = 0; branchMove2(true);  if drops != 11 { return 7 }
    drops = 0; branchMove2(false); if drops != 11 { return 8 }
    drops = 0; reinit();           if drops != 1011 { return 9 }
    drops = 0; wholeReassign();    if drops != 1111 { return 10 }
    drops = 0;
    let s = S(inner: R(id: 1), other: R(id: 10));
    let r = s.into();
    if drops != 10 { return 11 }
    0
}
