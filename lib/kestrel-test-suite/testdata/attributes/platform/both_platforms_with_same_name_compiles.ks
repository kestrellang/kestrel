// test: diagnostics
// stdlib: false

module Test

@platform(.darwin)
func value() -> lang.i64 { 1 }

@platform(.linux)
func value() -> lang.i64 { 2 }
