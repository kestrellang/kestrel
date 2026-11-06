// test: execution
// backends: cranelift,llvm
// expect-exit: 0

// `Never` conforms to every protocol, so an operator on a diverging operand
// type-checks: `(return 1) + 2` used to fail with "no member 'add' on Never".

module Test

func f(c: Bool) -> Int64 {
    if c { (return 1) + 2 } else { 3 }
}

@main
func main() -> lang.i64 {
    if f(true) != 1 { return 1 }
    if f(false) != 3 { return 2 }
    0
}
