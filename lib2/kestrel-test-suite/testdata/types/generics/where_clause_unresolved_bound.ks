// test: diagnostics
// stdlib: false

module Test

struct Set[T] where T: NonExistent { } // ERROR: cannot find type 'NonExistent' in this scope
