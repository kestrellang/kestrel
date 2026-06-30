// test: execution
// stdlib: true
// backends: cranelift,llvm
// expect-exit: 0

// Float32 shares the exact digit engine (24-bit significand decomposition).
// Regression for #161/#216 on the 32-bit width + bitPattern intrinsic.

module Test

import std.numeric.Float32
import std.numeric.UInt32
import std.text.(FormatOptions, FloatStyle)

func f(v: Float32) -> Float32 { v }

func fixedOpts(precision: Int64) -> FormatOptions {
    var opts = FormatOptions();
    opts.floatStyle = FloatStyle.Fixed;
    opts.precision = .Some(precision);
    opts
}

@main
func main() -> lang.i32 {
    if "\(f(0.5))" != "0.5" { return 1 };
    if "\(f(1.5))" != "1.5" { return 2 };
    if f(0.125).formatted(fixedOpts(2)) != "0.12" { return 3 };   // exact, half-even
    if f(2.5).formatted(fixedOpts(0)) != "2" { return 4 };         // half-even
    // bitPattern: 1.0f = 0x3F800000
    if f(1.0).bitPattern != UInt32(from: 1065353216) { return 5 };
    if Float32(bitPattern: UInt32(from: 1065353216)) != f(1.0) { return 6 };
    0
}
