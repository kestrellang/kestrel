// test: diagnostics
// stdlib: false

module Test

type Foo = lang.i64
type Foo = lang.str // ERROR: duplicate
