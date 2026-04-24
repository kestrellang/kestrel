// test: diagnostics
// stdlib: false

module Test

func decimal() -> lang.i64 { 42 }
func hex_lower() -> lang.i64 { 0xff }
func hex_upper() -> lang.i64 { 0XAB }
func binary() -> lang.i64 { 0b1010 }
func octal() -> lang.i64 { 0o755 }
func zero() -> lang.i64 { 0 }
func large() -> lang.i64 { 9223372036854775807 }
