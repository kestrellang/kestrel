// test: diagnostics
// stdlib: false

module Test

type Valid1 = lang.i64
type Valid2 = lang.str
type Cycle1 = Cycle2 // ERROR: circular type alias
type Cycle2 = Cycle1
