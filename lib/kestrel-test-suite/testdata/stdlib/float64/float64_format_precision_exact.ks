// test: execution
// stdlib: true
// backends: cranelift,llvm
// expect-exit: 0

// Regression (#161): the `:.n` precision path must round the STORED binary
// value with round-to-nearest-even, not double-round via float arithmetic.
// The old `value*10^n / round / trunc` pipeline produced 0.3332, 0.11, 0.4.

module Test

import std.numeric.Float64
import std.text.(FormatOptions, FloatStyle)

func f(v: Float64) -> Float64 { v }

func fixedOpts(precision: Int64) -> FormatOptions {
    var opts = FormatOptions();
    opts.floatStyle = FloatStyle.Fixed;
    opts.precision = .Some(precision);
    opts
}

@main
func main() -> lang.i32 {
    let third = f(1.0) / f(3.0);
    // Auto style (the `:.n` interpolation): small magnitudes stay fixed-point.
    if "\(third:.4)" != "0.3333" { return 1 };       // was 0.3332
    if "\(f(0.125):.2)" != "0.12" { return 2 };       // exact 0.125 → 0.12 (half-even); was 0.11
    if "\(f(0.35):.1)" != "0.3" { return 3 };         // stored 0.349999… → 0.3; was 0.4
    if "\(third:.15)" != "0.333333333333333" { return 4 };
    if "\(f(0.0):.3)" != "0.000" { return 5 };

    // Explicit Fixed style: exact round-to-nearest-even of the stored value.
    let a = f(123.456).formatted(fixedOpts(2));
    if a != "123.46" { return 6 };
    let b = f(2.5).formatted(fixedOpts(0));
    if b != "2" { return 7 };                          // half-even
    let c = f(0.5).formatted(fixedOpts(0));
    if c != "0" { return 8 };                          // half-even → 0
    let d = f(1.005).formatted(fixedOpts(2));
    if d != "1.00" { return 9 };                       // stored 1.00499…
    0
}
