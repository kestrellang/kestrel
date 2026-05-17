// test: diagnostics
// stdlib: false

module Test

@builtin(.Copyable)
protocol Copyable {}

@builtin(.Cloneable)
protocol Cloneable: Copyable {
    @builtin(.Clone)
    func clone() -> Self
}

func makeClone[T](item: T) -> T where T: Cloneable {
    item.clone()
}
