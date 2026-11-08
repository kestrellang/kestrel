// test: execution
// backends: cranelift,llvm
// expect-exit: 18

// A type-body conformance with a DEFAULTED protocol arg (`N: Mul`, i.e.
// `Mul[Self]`) plus an extension conformance to another instantiation
// (`extend N: Mul[Meters]`). The defaulted one is a real instantiation:
// `a.mul(b)` with `b: N` must select it (Output = N), the two `mul`s are not
// duplicates (E411), and each witness is checked against its own
// conformance's `Output` (no E632).
module Main

protocol Mul[Other = Self] {
    type Output
    consuming func mul(consuming other: Other) -> Output
}

struct N: Mul {
    var v: lang.i64
    type Mul.Output = N
    consuming func mul(consuming other: N) -> N { N(v: lang.i64_mul(self.v, other.v)) }
}

struct Meters { var v: lang.i64 }

extend N: Mul[Meters] {
    type Output = Meters
    consuming func mul(consuming other: Meters) -> Meters { Meters(v: lang.i64_mul(self.v, other.v)) }
}

@main
func main() -> lang.i64 {
    let a = N(v: 2);
    let b = N(v: 3);
    let c: N = a.mul(b);
    let d = N(v: 4).mul(Meters(v: 3));
    lang.i64_add(c.v, d.v)
}
