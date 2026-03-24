// test: diagnostics
// stdlib: false

module Test

struct Bad[T = lang.i64, U] {} // ERROR: with default must come after
