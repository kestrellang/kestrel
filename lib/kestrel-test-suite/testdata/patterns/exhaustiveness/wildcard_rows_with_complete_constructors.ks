// test: execution
// backends: cranelift,llvm
// expect-exit: 0

// A wildcard row does not hide the constructor rows around it: these arms
// cover every combination, so the match is exhaustive (it was reported as
// missing `_`).

module Test

enum Tri { case a case b case c }

func f(x: (Bool, Bool)) -> Int64 {
    match x {
        (true, false) => 1,
        (_, true) => 2,
        (false, false) => 4
    }
}

func g(x: (Tri, Bool)) -> Int64 {
    match x {
        (.a, _) => 1,
        (_, true) => 2,
        (.b, false) => 3,
        (.c, false) => 4
    }
}

@main
func main() -> lang.i64 {
    if f((true, false)) != 1 { return 1 }
    if f((false, true)) != 2 { return 2 }
    if f((false, false)) != 4 { return 3 }
    if g((.c, true)) != 2 { return 4 }
    if g((.c, false)) != 4 { return 5 }
    0
}
