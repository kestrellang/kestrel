// test: execution
// backends: cranelift,llvm
// expect-exit: 0

// `return return x` returns x: the inner return terminates the block, so the
// outer one has nothing left to do (it used to emit a second terminator).

module Test

func f() -> Int64 { return return 7; }

@main
func main() -> lang.i64 {
    if f() != 7 { return 1 }
    0
}
