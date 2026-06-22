// test: execution
// stdlib: true
// backends: cranelift,llvm
// expect-exit: 0

// Regression (#216 + #161): exact integer digit generation across the full
// magnitude range. The old scientific path used `powi(exponent)` which
// underflowed to 0 on subnormals → inf → `cast_f64_i64` saturated to
// Int64.max (9223372036854775807) as the mantissa digits; large magnitudes
// lost the last digit (2^63 → 9.223371e18).

module Test

import std.numeric.Float64

func f(v: Float64) -> Float64 { v }

@main
func main() -> lang.i32 {
    // smallest positive subnormal 2^-1074 ≈ 4.9406564584…e-324
    if "\(f(5.0e-324))" != "4.940656e-324" { return 1 };   // was 9223372036854775807e-323 garbage
    if "\(f(9.2e-305))" != "9.2e-305" { return 2 };        // was 9.199999e-305 (truncated)
    if "\(f(1.0e-300))" != "1e-300" { return 3 };
    if "\(f(1.0e-10))" != "1e-10" { return 4 };
    // large magnitude: 2^63 exactly
    if "\(f(9223372036854775808.0))" != "9.223372e18" { return 5 };  // was 9.223371e18
    if "\(f(1.0e20))" != "1e20" { return 6 };
    if "\(f(123456789.123456789))" != "1.234568e8" { return 7 };
    0
}
