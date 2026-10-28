// test: diagnostics
// stdlib: false

// Copy-drift #2 convergence (2026-06-10): a mixed tuple (Copyable element +
// Cloneable element) folds to Cloneable, so it satisfies BOTH `T: Copyable`
// and `T: Cloneable` bounds. Previously the solver required ALL elements
// Cloneable for the Cloneable bound and classified mixed tuples plain
// Copyable.

module Test

@builtin(.Copyable)
protocol Copyable {}

@builtin(.Cloneable)
protocol Cloneable: Copyable {
    func clone() -> Self
}

struct Plain {
    var x: lang.i64
}

struct Heap: Cloneable {
    var value: lang.i64

    func clone() -> Heap {
        Heap(value: self.value)
    }
}

func needsCopyable[T](item: T) -> lang.i64 where T: Copyable {
    0
}

func needsCloneable[T](item: T) -> lang.i64 where T: Cloneable {
    0
}

func test() -> lang.i64 {
    let mixed = (Plain(x: 1), Heap(value: 2));
    let viaCopyable = needsCopyable(mixed);
    needsCloneable(mixed)
}
