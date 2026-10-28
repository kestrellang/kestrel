// test: execution
// stdlib: true
// backends: cranelift,llvm
// expect-exit: 0

// Regression (#216): parsing accumulates digits into a big integer and rounds
// the exact decimal to nearest (round-even), instead of `result * 10.pow(exp)`
// in float arithmetic, which overflowed 10^k to infinity -> 0 for long forms.

module Test

import std.numeric.Float64

func parse(s: String) -> Float64 {
    if let .Some(v) = Float64(parsing: s) { v } else { Float64.nan }
}

@main
func main() -> lang.i32 {
    // #216: long-mantissa form is representable (~9.22e-305), must not be 0.
    let long = parse("9223372036854775807e-323");
    if long == 0.0 { return 1 };
    if "\(long)" != "9.223372036854775e-305" { return 2 };

    // parse -> print is an identity on canonical shortest forms.
    if "\(parse("0.1"))" != "0.1" { return 3 };
    if "\(parse("0.30000000000000004"))" != "0.30000000000000004" { return 4 };
    if "\(parse("3.14159"))" != "3.14159" { return 5 };
    if "\(parse("-2.5"))" != "-2.5" { return 6 };
    if "\(parse("100"))" != "100" { return 7 };

    // smallest positive subnormal round-trips through parse.
    if "\(parse("5e-324"))" != "5e-324" { return 8 };
    // exact powers of ten.
    if "\(parse("1e308"))" != "1e308" { return 9 };

    // overflow -> infinity, not garbage / zero.
    if parse("1e400").isInfinite == false { return 10 };
    // tiny -> zero (underflow below half the smallest subnormal).
    if parse("1e-400") != 0.0 { return 11 };
    // absurd exponent must not wrap Int64 and flip inf<->0 (overflow cap).
    if parse("1e9223372036854775808").isInfinite == false { return 14 };
    if parse("1e-9223372036854775809") != 0.0 { return 15 };

    // subnormal round-to-nearest-even at the smallest grid steps.
    if parse("2e-324") != 0.0 { return 16 };          // below half-ulp -> 0
    if "\(parse("7.5e-324"))" != "1e-323" { return 17 };  // rounds up to 2*2^-1074

    // value equality after parse (not just string form).
    if parse("2.5") != 2.5 { return 12 };
    if parse("0.5") != 0.5 { return 13 };
    0
}
