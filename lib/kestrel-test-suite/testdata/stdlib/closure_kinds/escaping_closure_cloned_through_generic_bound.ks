// test: execution
// backends: cranelift,llvm
// expect-exit: 0

// An `escaping` closure is `Cloneable` (a shared environment). Cloning one
// through a generic `T: Cloneable` bound retains the environment; there is
// no `clone` witness function to call (it used to fail post-mono verify).

module Test

func gen[T](x: T) -> escaping () -> T where T: Cloneable {
    { x.clone() }
}

@main
func main() -> lang.i64 {
    let outer = gen(gen("q"));
    let inner = outer();
    let s = inner();
    if s.isEqual(to: "q") == false { return 1 }
    let again = outer()();
    if again.isEqual(to: "q") == false { return 2 }
    0
}
