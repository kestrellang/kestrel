// test: diagnostics
// stdlib: true

module Test

func accept[T](consuming x: T) where T: not Copyable { }

func forward[T](consuming x: T) where T: not Copyable {
    accept(x);  // x is moved here, that's fine
}
