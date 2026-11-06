// test: execution
// backends: cranelift,llvm
// expect-stdout: [0x1.ffp+7] [0X1.FFP+7] [0x0p+0] [0x1p+0] [0x1.8p+1] [-0x1p-1] [0x0.0000000000001p-1022]
// expect-exit: 0

// `:x` / `:X` on a float prints exact hexadecimal floating point (C's `%a`).
// It used to be ignored silently.

module Test

@main
func main() {
    let a = 255.5;
    let z = 0.0;
    let one: Float32 = 1.0;
    let three = 3.0;
    let half = -0.5;
    let tiny = 4.9406564584124654e-324;
    println("[\(a:x)] [\(a:X)] [\(z:x)] [\(one:x)] [\(three:x)] [\(half:x)] [\(tiny:x)]");
}
