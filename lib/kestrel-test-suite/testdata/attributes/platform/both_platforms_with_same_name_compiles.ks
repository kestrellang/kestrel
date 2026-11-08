// test: diagnostics
// stdlib: false

module Test

@platform(.Darwin)
func value() -> lang.i64 { 1 }

@platform(.Linux)
func value() -> lang.i64 { 2 }
