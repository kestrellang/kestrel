// test: execution
// stdlib: true
// expect-exit: 0
//
// A static requirement named through a type parameter, `T.make`, used as a
// function value and called. Today the program compiles but segfaults
// (exit 139) at runtime, both through a `let` and when passed to a
// function. Found 2026-11-01 (pre-existing at c75b3b71) while fixing static
// method values in `escaping` slots.

module Test

protocol Make {
    static func make(n: Int64) -> Self
    func n() -> Int64
}

struct A: Make {
    var v: Int64
    static func make(n: Int64) -> A { A(v: n * 3) }
    func n() -> Int64 { self.v }
}

func apply[T](f: (Int64) -> T, n: Int64) -> T { f(n) }

func direct[T](n: Int64) -> Int64 where T: Make {
    let f = T.make;
    f(n).n()
}

func passed[T](n: Int64) -> Int64 where T: Make { apply(T.make, n).n() }

@main
func main() -> Int64 {
    if direct[A](4) != 12 { return 1 }
    if passed[A](5) != 15 { return 2 }
    0
}
