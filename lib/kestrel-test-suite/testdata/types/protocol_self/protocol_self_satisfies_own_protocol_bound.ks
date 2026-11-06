// test: execution
// backends: cranelift,llvm
// expect-exit: 0

// A protocol's abstract `Self` conforms to the protocol itself, so it can be
// passed to a `where T: P` function. `T: not Copyable` because `Self` is not
// Copyable by default (move-only types may conform to `P`).

module Test

protocol P { func f() -> Int64 }

extend P {
    func g() -> Int64 { helper(self) }
}

func helper[T](x: T) -> Int64 where T: P, T: not Copyable { x.f() * 2 }

struct S: P { func f() -> Int64 { 21 } }

@main
func main() -> lang.i64 {
    if S().g() != 42 { return 1 }
    0
}
