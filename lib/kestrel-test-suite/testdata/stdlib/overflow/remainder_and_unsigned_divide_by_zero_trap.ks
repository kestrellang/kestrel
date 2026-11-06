// test: execution
// stdlib: true
// expect-exit: -1

// `%` and unsigned `/` by zero trap like signed `/`. At -O0 (Cranelift) they
// used to reach the native x86 divide and die with SIGFPE instead of the
// explicit trap every other runtime check uses; the harness sees any signal
// as -1, so this documents the trap itself.
module Test

@main
func main() -> lang.i64 {
    let zero = Int64(parsing: "0")!;
    let r = 7 % zero;
    if r == 0 { return 1 }
    0
}
