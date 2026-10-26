// test: execution
// backends: cranelift,llvm
// expect-exit: 0

// A generic struct with an `escaping` closure field whose signature mentions
// an associated type (`escaping (I.A) -> Bool`) gets a clone shim like any
// other closure field. It used to be skipped and failed the MIR verifier
// with "Copyable contains non-Copyable field".

module Test

protocol P { type A }
struct S: P { type A = Int64; }
struct W[I] where I: P {
    var i: I;
    let f: escaping (I.A) -> Bool;
}

@main
func main() -> lang.i64 {
    let w = W(i: S(), f: { (x: Int64) in x > 0 });
    let copy = w;
    if not (copy.f)(1) { return 1 }
    if (w.f)(-1) { return 2 }
    0
}
