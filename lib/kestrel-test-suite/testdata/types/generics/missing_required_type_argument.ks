// test: diagnostics
// stdlib: false

module Test

struct Map[K, V] { }
type BadMap = Map[lang.i64]; // ERROR: too few type arguments
