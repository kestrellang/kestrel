// test: execution
// backends: cranelift,llvm
// expect-exit: 0

// Documented edge cases of float `round()`/`abs()` and integer `gcd`:
// - `round()` breaks ties away from zero (both backends used ties-to-even);
// - `abs()` clears the sign bit, so `-0.0` becomes `+0.0`;
// - `gcd` is non-negative, even with `minValue` (`abs()` wraps there).

module Test

@main
func main() -> lang.i64 {
    if (2.5).round() != 3.0 { return 1 }
    if (-2.5).round() != -3.0 { return 2 }
    if (0.5).round() != 1.0 { return 3 }
    if (0.49999999999999994).round() != 0.0 { return 4 }
    if (1.4).round() != 1.0 { return 5 }
    if (-1.6).round() != -2.0 { return 6 }
    let f: Float32 = 2.5;
    if f.round() != 3.0 { return 7 }

    let nz = (0.0).negate();
    if nz.abs().bitPattern != 0 { return 8 }
    if (-3.5).abs() != 3.5 { return 9 }

    if Int64.minValue.gcd(6) != 2 { return 10 }
    if (-12).gcd(8) != 4 { return 11 }
    if Int8.minValue.gcd(6) != 2 { return 12 }
    0
}
