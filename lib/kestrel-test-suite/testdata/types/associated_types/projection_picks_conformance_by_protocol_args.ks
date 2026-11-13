// test: execution
// stdlib: true
// expect-exit: 0
// expect-stdout: 703\ntrue\nfalse\n
//
// `Int64` conforms to `VIdx[VA]` and `VIdx[UB]` with different `Output`s.
// `I.Output where I: VIdx[UB]` must project through the `VIdx[UB]`
// conformance, not whichever conformance of `Int64` is found first.

module Test

struct VA { var n: Int64 }
struct UB { var n: Int64 }

protocol VIdx[W] {
    type Output
    @sugarOnly
    static func read(consuming index: Self, view: W) -> Output
}

extend Int64: VIdx[VA] {
    type Output = Int64
    static func read(consuming index: Int64, view: VA) -> Int64 { view.n * 100 + index }
}

extend Int64: VIdx[UB] {
    type Output = Bool
    static func read(consuming index: Int64, view: UB) -> Bool { index % 2 == 0 }
}

extend VA { subscript[I](index: I) -> I.Output where I: VIdx[VA] { I.read(index, self) } }
extend UB { subscript[I](index: I) -> I.Output where I: VIdx[UB] { I.read(index, self) } }

@main
func main() {
    let v = VA(n: 7);
    let a: Int64 = v(3);
    print("\(a)\n");
    let u = UB(n: 1);
    let b: Bool = u(4);
    print("\(b)\n");
    let c: Bool = u(3);
    print("\(c)\n");
}
