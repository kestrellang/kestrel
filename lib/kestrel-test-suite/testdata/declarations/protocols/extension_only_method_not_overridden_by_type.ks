// test: execution
// backends: cranelift,llvm
// expect-exit: 0

// A method only a protocol extension declares (not a requirement) is not
// overridable: through a generic `T: P` it calls the extension's version even
// when the type has its own method of the same name. It used to dispatch to
// the type's method.

module Test

protocol P { func f() -> Int64 }
extend P { func g() -> Int64 { 1 } }

struct S: P {
    func f() -> Int64 { 2 }
    func g() -> Int64 { 50 }
}

func run[T](x: T) -> Int64 where T: P { x.g() }

@main
func main() -> lang.i64 {
    if run(S()) != 1 { return 1 }
    // A direct call on the concrete type still sees its own method.
    if S().g() != 50 { return 2 }
    0
}
