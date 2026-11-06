// test: execution
// backends: cranelift,llvm
// expect-exit: 0

// Range patterns tile an integer type's own domain: `0..=255` covers all of
// `UInt8`, and `..<0 | 0..` covers all of `Int8`. They used to be checked
// against `Int64`'s domain and rejected as non-exhaustive.

module Test

func byte(x: UInt8) -> Int64 { match x { 0..=127 => 1, 128..=255 => 2 } }
func word(x: UInt64) -> Int64 { match x { 0..=9223372036854775807 => 1, 9223372036854775808.. => 2 } }
func sign(x: Int8) -> Int64 { match x { ..<0 => 1, 0.. => 2 } }

@main
func main() -> lang.i64 {
    let b: UInt8 = 200;
    let w: UInt64 = 18446744073709551615;
    let s: Int8 = -1;
    if byte(b) != 2 { return 1 }
    if word(w) != 2 { return 2 }
    if sign(s) != 1 { return 3 }
    0
}
