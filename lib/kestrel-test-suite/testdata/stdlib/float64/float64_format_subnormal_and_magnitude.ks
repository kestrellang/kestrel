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
import std.text.(FormatOptions, FloatStyle)

func f(v: Float64) -> Float64 { v }

func sciOpts(precision: Int64) -> FormatOptions {
    var opts = FormatOptions();
    opts.floatStyle = FloatStyle.Scientific;
    opts.precision = .Some(precision);
    opts
}

@main
func main() -> lang.i32 {
    // Default-print is shortest-round-trip: the smallest positive subnormal
    // 2^-1074 prints faithfully as "5e-324" (was Int64.max-mantissa garbage).
    if "\(f(5.0e-324))" != "5e-324" { return 1 };          // was 9223372036854775807e-323 garbage
    if "\(f(9.2e-305))" != "9.2e-305" { return 2 };        // was 9.199999e-305 (truncated)
    if "\(f(1.0e-300))" != "1e-300" { return 3 };
    if "\(f(1.0e-10))" != "1e-10" { return 4 };
    // 2^63: shortest round-trips to the full true digits (was 9.223371e18).
    if "\(f(9223372036854775808.0))" != "9.223372036854776e18" { return 5 };
    if "\(f(1.0e20))" != "1e20" { return 6 };

    // Explicit precision-6 scientific: exact rounding, no underflow/saturation.
    let a = f(5.0e-324).formatted(sciOpts(6));
    if a != "4.940656e-324" { return 7 };                  // exact 7-sig of 2^-1074
    let b = f(9223372036854775808.0).formatted(sciOpts(6));
    if b != "9.223372e18" { return 8 };
    let c = f(123456789.123456789).formatted(sciOpts(6));
    if c != "1.234568e8" { return 9 };
    0
}
