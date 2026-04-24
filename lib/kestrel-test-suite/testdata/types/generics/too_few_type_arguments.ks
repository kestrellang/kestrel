// test: diagnostics
// stdlib: false

module Test

struct Map[K, V] { }
type Bad = Map[lang.i64]; // ERROR: too few type arguments
