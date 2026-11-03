// test: execution
// backends: cranelift,llvm
// stdlib: true
// expect-stdout: big\n

// An `if` with no `else` is unit-typed, even when its block's tail is a
// non-unit value (`println` returns a `Result`). The tail value is
// discarded; lowering used to ICE on the type mismatch.
module Test

@main
func main() {
    let v = true;
    if v { println("big") }
}
