// test: execution
//
// Regression guard: a fully-concrete extension conformance of a generic
// struct instantiation to a *parameterized* protocol
// (`extend Box[Int64]: Reader[Int64]`) dispatched through a generic
// bound. This single-module shape works; the equivalent CROSS-MODULE
// shape (protocol in std.numeric, `extend ClosedRange[Int64]:
// RandomBounds[Int64]` targeting a std.core struct) failed post-mono
// with "no matching conformance for this instantiation" on 2026-07-23,
// which is why std uses free-parameter conformances
// (`extend ClosedRange[T]: RandomBounds[T] where ...`) instead.

module Test

protocol Reader[B] {
    func value() -> B
}

struct Box[T] where T: Comparable {
    var item: T;
}

extend Box[Int64]: Reader[Int64] {
    public func value() -> Int64 { self.item }
}

func readThrough[R](source: R) -> Int64 where R: Reader[Int64] {
    source.value()
}

@main
func main() -> lang.i64 {
    let box = Box[Int64](item: 41);
    if readThrough(box) != 41 { return 1 }
    0
}
