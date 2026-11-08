// test: diagnostics
// stdlib: false
//
// `@align` takes a power of two from 1 to 4096 (E909), and only on structs
// and enums (E900).

module Main

@align(12) // ERROR: '@align(12)' must be a power of two between 1 and 4096
struct NotPowerOfTwo { var v: lang.i64 }

@align(0) // ERROR: '@align(0)' must be a power of two
struct Zero { var v: lang.i64 }

@align(8192) // ERROR: '@align(8192)' must be a power of two
struct TooLarge { var v: lang.i64 }

@align(4096)
struct Largest { var v: lang.i64 }

@align(1)
struct Smallest { var v: lang.i64 }

@align(8) // ERROR: '@align' cannot be applied to a function; it applies to structs and enums
func notAType() {}

@align // ERROR: '@align' needs an alignment
struct Missing { var v: lang.i64 }
