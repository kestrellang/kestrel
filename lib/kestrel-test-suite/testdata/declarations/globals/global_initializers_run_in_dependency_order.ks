// test: execution
// backends: cranelift,llvm
// expect-exit: 0

// Global initializers run after the globals they read, whatever the
// declaration order: `a` reads `b` before `b` is declared. It used to run in
// declaration order and read `b` as 0.

module Test

let a: Int64 = b + 1;
let b: Int64 = c * 2;
let c: Int64 = 20;

@main
func main() -> lang.i64 {
    if c != 20 { return 1 }
    if b != 40 { return 2 }
    if a != 41 { return 3 }
    0
}
