// test: diagnostics
// stdlib: false

module Test

struct Box[T] { }
type Alias = Box; // ERROR: too few type arguments
