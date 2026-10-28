// test: diagnostics
// stdlib: true

// Stage 2d containment: a member-RESULT instantiation is a formation
// site — the solver-side Static wellformedness fires when MEMBER
// dispatch materializes a Static-bounded container at a ref type arg.
// Before this gate, `arr.refs().collect()` BUILT an `Array[&Int64]`
// out of inference (heap storage of refs — the line that never moves);
// the stdlib collect() now rejects identically (its diagnostic anchors
// at iterator.ks's `-> Array[Item]`, unannotatable here — this test
// pins the same gate on a test-file method signature). Free-function
// generic calls still bypass (the instantiated-signature
// wellformedness residual, same family as the Copyable mono gap).
module Test

struct Sink[T] {
    var marker: Int64
}

struct Trapper {
    var marker: Int64

    func trap[I](it: I) -> Sink[I.Item] where I: Iterator { // ERROR: !: Static
        Sink(marker: 0)
    }
}

struct RefIter {
    var v: Int64
}

extend RefIter: Iterator {
    type Item = &Int64

    mutating func next() -> Optional[&Int64] {
        .None
    }
}

@main
func main() {
    let it = RefIter(v: 1);
    let s = Trapper(marker: 0).trap(it);
}
