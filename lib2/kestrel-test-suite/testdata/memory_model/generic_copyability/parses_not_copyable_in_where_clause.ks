// test: diagnostics
// stdlib: true

module Test

func process[T](consuming x: T) where T: not Copyable { }
