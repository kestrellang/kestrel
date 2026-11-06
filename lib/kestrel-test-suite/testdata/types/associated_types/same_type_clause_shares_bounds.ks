// test: execution
// backends: cranelift,llvm
// expect-exit: 0

// `where T.A = U.A` makes the two projections one type, so a bound stated
// on either holds for both: `T.A: Equatable` lets `x.a() == y.a()` compare
// a `T.A` with a `U.A`. The bound used to be found only on `T.A`.

module Test

protocol P {
    type A
    func a() -> A
}

struct S: P {
    type A = String
    func a() -> String { "hello" }
}

struct S2: P {
    type A = String
    func a() -> String { "hellp" }
}

func same[T, U](x: T, y: U) -> Bool where T: P, U: P, T.A = U.A, T.A: Equatable {
    x.a() == y.a()
}

// The bound may sit on either side of the equality.
func sameFlipped[T, U](x: T, y: U) -> Bool where T: P, U: P, T.A = U.A, U.A: Equatable {
    x.a() == y.a()
}

@main
func main() -> lang.i64 {
    if not same(S(), S()) { return 1 }
    if same(S(), S2()) { return 2 }
    if not sameFlipped(S2(), S2()) { return 3 }
    0
}
