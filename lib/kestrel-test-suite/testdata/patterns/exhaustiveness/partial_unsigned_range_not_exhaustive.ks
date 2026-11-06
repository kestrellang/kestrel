// test: diagnostics

// `0..=254` misses `255` of `UInt8`.

module Test

func byte(x: UInt8) -> Int64 {
    match x { 0..=254 => 1 } // ERROR: non-exhaustive
}
