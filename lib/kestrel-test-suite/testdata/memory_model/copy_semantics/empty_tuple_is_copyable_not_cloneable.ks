// test: diagnostics
// stdlib: false

// Copy-drift #2 convergence (2026-06-10): the empty tuple folds to Copyable
// (no Cloneable elements), so it satisfies `T: Copyable` but NOT
// `T: Cloneable`. Previously the solver's all-elements rule made `()`
// vacuously Cloneable.

module Test

@builtin(.Copyable)
protocol Copyable {}

@builtin(.Cloneable)
protocol Cloneable: Copyable {
    func clone() -> Self
}

func nothing() {}

func needsCopyable[T](item: T) -> lang.i64 where T: Copyable {
    0
}

func needsCloneable[T](item: T) -> lang.i64 where T: Cloneable {
    0
}

func test() -> lang.i64 {
    let viaCopyable = needsCopyable(nothing());
    needsCloneable(nothing()) // ERROR: does not conform to protocol
}
