// test: execution
// backends: cranelift,llvm
// expect-stdout: [3.14] [12.3] [2] [1000.00] [12345678.9]
// expect-exit: 0

// An explicit `:.N` precision is N decimal places in fixed notation. It
// used to switch to scientific notation with N significant digits once the
// value had N or more integer digits (`12.34:.1` printed "1.2e1").

module Test

@main
func main() {
    let pi = 3.14159; let a = 2.5; let b = 999.9999; let d = 12.34; let big = 12345678.9;
    println("[\(pi:.2)] [\(d:.1)] [\(a:.0)] [\(b:.2)] [\(big:.1)]");
}
