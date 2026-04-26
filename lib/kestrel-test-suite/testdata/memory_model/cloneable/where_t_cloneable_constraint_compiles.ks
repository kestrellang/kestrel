// test: diagnostics
// stdlib: false

module Test

@builtin(.Copyable)
protocol Copyable {}

@builtin(.Cloneable)
protocol Cloneable: Copyable {
    func clone() -> Self
}

func duplicate[T](item: T) -> (T, T) where T: Cloneable {
    let copy = item.clone();
    (item, copy)
}
