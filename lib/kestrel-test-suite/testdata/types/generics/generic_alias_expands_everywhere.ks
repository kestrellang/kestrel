// test: execution
// backends: cranelift,llvm
// expect-exit: 0

// A generic alias is expanded where it is named, so every stage sees the
// real type: a `Pair[T]` return accepts a tuple, and `.1` indexes it. It used
// to stay an unexpanded alias outside type equality ("cannot index into
// non-tuple type 'Pair[Int64]'", or a mismatch against `(T, T)`).

module Test

type Pair[T] = (T, T);
type Grid[T] = Array[Pair[T]];

func mk[T](x: T) -> Pair[T] { (x, x) }

func first(g: Grid[Int64]) -> Int64 { g(0).1 }

@main
func main() -> lang.i64 {
    let p = mk("a");
    if p.1 != "a" { return 1 }
    let q: Pair[Int64] = (3, 4);
    if q.0 + q.1 != 7 { return 2 }
    if first([(5, 6)]) != 6 { return 3 }
    0
}
