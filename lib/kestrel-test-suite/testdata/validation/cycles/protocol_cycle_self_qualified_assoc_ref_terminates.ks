// test: diagnostics
// stdlib: false

module Test

// Degenerate one-protocol cycle via a qualified path. Same crash as the
// two-way case: the associated-type walk revisited `Foo` forever.
protocol Foo: Test.Foo {} // ERROR(E459)

func take[T](x: T.Missing) -> () where T: Foo {} // ERROR: cannot find type 'T.Missing' in this scope
