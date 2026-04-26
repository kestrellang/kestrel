// test: diagnostics
// stdlib: true

module Test

protocol Displayable {}

func process[T, U](consuming x: T, y: U) where T: not Copyable, U: Displayable { }
