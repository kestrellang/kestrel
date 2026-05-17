// test: diagnostics
// stdlib: false

module Test

struct Map[K, V = lang.str] { }
type Inferred = Map; // ERROR: too few type arguments
