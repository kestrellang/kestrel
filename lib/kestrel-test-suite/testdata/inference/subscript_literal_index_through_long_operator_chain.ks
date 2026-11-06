// test: execution
// stdlib: true
// backends: cranelift,llvm
// expect-stdout: 14

// `xs(0)` is generic over its index, so the element type waits on the `0`
// literal. A method call on a value built from it must default that literal
// however far back it is: three `+` put it five constraints behind `sum`,
// which used to stop the walk short and report a false E106 ("the type of
// this value must be known here to look up 'rotateLeft'", MD5.compress).
module Test

func mix(xs: Array[UInt32]) -> UInt32 {
    let a = xs(0);
    let f: UInt32 = 2;
    let sum = a + f + f + f;
    sum.rotateLeft(by: 1)
}

@main
func main() {
    let xs: Array[UInt32] = [1, 9];
    print(mix(xs));
}
