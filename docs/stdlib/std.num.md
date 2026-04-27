# std.num

## typealias `Float`

```kestrel
public type Float = Float64
```

Default floating-point type — alias for `Float64`. Reach for `Float`
when you want the recommended precision/performance trade-off; reach for
`Float32` only when you specifically need 32-bit storage.

_Defined in `lang/std/num/float64.ks`._

## struct `Float32`

```kestrel
public struct Float32 { /* private fields */ }
```

A 32-bit IEEE 754 single-precision float.

Range is approximately ±3.4×10^38 with 6-9 significant decimal
digits. Float literals without a type annotation default to `Float64`;
annotate the binding to pick `Float32`. The type is `FFISafe` and lays out
as a single `lang.f32`.

### Examples

```
let pi = Float64.pi;
let area = pi * radius * radius;
let s = area.format(options: .{precision: 2});  // "314.16"
```

```
let x = 3.14;          // Float64
let y: Float32 = 3.14; // Float32
```

### Special Values

- `nan` — Not-a-Number, result of `0.0 / 0.0`, `sqrt(-1)`, etc.
- `infinity` / `-infinity` — overflow or `1.0 / 0.0`.
- Negative zero compares equal to positive zero but produces `-infinity`
  when used as a divisor.

NaN comparisons are surprising: `nan == nan` is false and every ordered
comparison against NaN is false. Use `isNaN` to test, never `== nan`. Any
arithmetic with NaN propagates NaN.

### Representation

A single `lang.f32` field holding the raw IEEE 754 bit pattern.

_Defined in `lang/std/num/float32.ks`._

### Members

#### initializer `Default`

```kestrel
public init()
```

Creates the zero value, satisfying `Defaultable`.

_Defined in `lang/std/num/float32.ks`._

#### initializer `Float Literal`

```kestrel
public init(floatLiteral: lang.f64)
```

Compiler-emitted bridge for floating-point literals via
`ExpressibleByFloatLiteral`. Rarely called directly.

##### Examples

```
let x: Float64 = 3.14;                  // implicit
let y = Float64(floatLiteral: 3.14);    // explicit
```

_Defined in `lang/std/num/float32.ks`._

#### initializer `From Float`

```kestrel
public init(from: Float64)
```

Converts between `Float32` and `Float64`. The 32→64 direction is
exact; the 64→32 direction rounds and may overflow to ±infinity.

##### Examples

```
let f32: Float32 = 3.14;
let f64 = Float64(from: f32);
```

_Defined in `lang/std/num/float32.ks`._

#### initializer `From Int`

```kestrel
public init(from: Int64)
```

Converts an `Int64` to a float. Values with magnitude greater than
2^53 lose low-order bits.

##### Examples

```
let n: Int64 = 42;
let f = Float64(from: n);  // 42.0
```

_Defined in `lang/std/num/float32.ks`._

#### initializer `From Raw`

```kestrel
init(raw: lang.f32)
```

Wraps an existing `lang.f32` bit pattern. Internal; used
by intrinsics.

_Defined in `lang/std/num/float32.ks`._

#### initializer `Int Literal`

```kestrel
public init(intLiteral: lang.i64)
```

Bridge that lets bare integer literals appear where a float is
expected. Conversion is exact up to ±2^53; larger magnitudes round.

##### Examples

```
let x: Float64 = 42;     // 42.0
let y = 3.14 + 1;        // 4.14 — `1` widened to Float64
```

_Defined in `lang/std/num/float32.ks`._

#### function `abs`

```kestrel
public func abs() -> Float32
```

Absolute value — clears the sign bit. NaN stays NaN; `-0.0` becomes
`+0.0`.

_Defined in `lang/std/num/float32.ks`._

#### function `acos`

```kestrel
public func acos() -> Float32
```

Arc cosine, result in radians on `[0, π]`. Returns NaN outside
`[-1.0, 1.0]`.

_Defined in `lang/std/num/float32.ks`._

#### function `acosh`

```kestrel
public func acosh() -> Float32
```

Inverse hyperbolic cosine. Returns NaN for inputs less than `1.0`.

_Defined in `lang/std/num/float32.ks`._

#### function `asin`

```kestrel
public func asin() -> Float32
```

Arc sine, result in radians on `[-π/2, π/2]`. Returns NaN outside
`[-1.0, 1.0]`.

_Defined in `lang/std/num/float32.ks`._

#### function `asinh`

```kestrel
public func asinh() -> Float32
```

Inverse hyperbolic sine. Defined on all real inputs.

_Defined in `lang/std/num/float32.ks`._

#### function `atan`

```kestrel
public func atan() -> Float32
```

Arc tangent, result in radians on `[-π/2, π/2]`. For full-quadrant
recovery use `atan2`.

_Defined in `lang/std/num/float32.ks`._

#### function `atan2`

```kestrel
public func atan2(Float32) -> Float32
```

Two-argument arctangent — angle of the point `(x, self)` measured
from the positive x-axis, on `[-π, π]`. Disambiguates quadrant where
`atan` cannot.

##### Examples

```
(1.0).atan2(x: 1.0);    //  π/4   (Q1)
(1.0).atan2(x: -1.0);   //  3π/4  (Q2)
(-1.0).atan2(x: -1.0);  // -3π/4  (Q3)
(-1.0).atan2(x: 1.0);   // -π/4   (Q4)
```

_Defined in `lang/std/num/float32.ks`._

#### function `atanh`

```kestrel
public func atanh() -> Float32
```

Inverse hyperbolic tangent. NaN outside `(-1.0, 1.0)`; `±inf` at ±1.

_Defined in `lang/std/num/float32.ks`._

#### function `cbrt`

```kestrel
public func cbrt() -> Float32
```

Real cube root. Defined for negatives — `(-8.0).cbrt() == -2.0`.

_Defined in `lang/std/num/float32.ks`._

#### function `ceil`

```kestrel
public func ceil() -> Float32
```

Smallest integer ≥ `self`. Rounds toward `+infinity`.

##### Examples

```
(3.2).ceil();   //  4.0
(-3.7).ceil();  // -3.0
```

_Defined in `lang/std/num/float32.ks`._

#### function `clamp`

```kestrel
public func clamp(Float32, Float32) -> Float32
```

Clamps `self` into `[min, max]`. NaN passes through unchanged. Caller
must ensure `min <= max`.

##### Examples

```
(0.5).clamp(min: 0.0, max: 1.0);   // 0.5
(-0.5).clamp(min: 0.0, max: 1.0);  // 0.0
(1.5).clamp(min: 0.0, max: 1.0);   // 1.0
```

_Defined in `lang/std/num/float32.ks`._

#### function `copysign`

```kestrel
public func copysign(from: Float32) -> Float32
```

Returns a value with `self`'s magnitude and `other`'s sign — i.e. an
IEEE 754 `copysign`. Useful for unbiased rounding tricks.

_Defined in `lang/std/num/float32.ks`._

#### function `cos`

```kestrel
public func cos() -> Float32
```

Cosine of `self` in radians.

_Defined in `lang/std/num/float32.ks`._

#### function `cosh`

```kestrel
public func cosh() -> Float32
```

Hyperbolic cosine.

_Defined in `lang/std/num/float32.ks`._

#### field `e`

```kestrel
public static var e: Float32 { get }
```

Euler's number `e` ≈ 2.71828182845904… — base of the natural logarithm.

_Defined in `lang/std/num/float32.ks`._

#### field `epsilon`

```kestrel
public static var epsilon: Float32 { get }
```

Machine epsilon — the smallest `e` such that `1.0 + e != 1.0`,
≈ 1.1920929e-7.

Useful as a tolerance in approximate comparisons; scale by the
operand magnitude for relative-error checks.

##### Examples

```
func almostEqual(a: Float64, b: Float64) -> Bool {
    (a - b).abs() < Float64.epsilon * a.abs().max(b.abs());
}
```

_Defined in `lang/std/num/float32.ks`._

#### function `exp`

```kestrel
public func exp() -> Float32
```

`e^self` via libm. `(-inf).exp()` is `0.0`; `(inf).exp()` is `inf`.

_Defined in `lang/std/num/float32.ks`._

#### function `exp2`

```kestrel
public func exp2() -> Float32
```

`2^self`. Useful for binary scaling.

_Defined in `lang/std/num/float32.ks`._

#### function `expm1`

```kestrel
public func expm1() -> Float32
```

`e^self - 1`, computed without the cancellation that hurts
`self.exp() - 1.0` for small `self`.

_Defined in `lang/std/num/float32.ks`._

#### function `floor`

```kestrel
public func floor() -> Float32
```

Largest integer ≤ `self`. Rounds toward `-infinity`.

##### Examples

```
(3.7).floor();   //  3.0
(-3.2).floor();  // -4.0
```

_Defined in `lang/std/num/float32.ks`._

#### function `fma`

```kestrel
public func fma(Float32, Float32) -> Float32
```

Fused multiply-add — `(self * a) + b` with a single rounding step.
More accurate (and often faster) than separate `multiply`/`add`.

##### Examples

```
(2.0).fma(a: 3.0, b: 4.0);   // 10.0
```

_Defined in `lang/std/num/float32.ks`._

#### function `fract`

```kestrel
public func fract() -> Float32
```

Fractional part — `self - self.trunc()`. Sign matches `self`.

##### Examples

```
(3.7).fract();   //  0.7
(-3.7).fract();  // -0.7
```

_Defined in `lang/std/num/float32.ks`._

#### function `hypot`

```kestrel
public func hypot(Float32) -> Float32
```

Hypotenuse — `sqrt(self² + other²)`, computed via libm in a way that
avoids intermediate overflow when one operand is very large.

##### Examples

```
(3.0).hypot(other: 4.0);  // 5.0
```

_Defined in `lang/std/num/float32.ks`._

#### field `infinity`

```kestrel
public static var infinity: Float32 { get }
```

Positive infinity. Produced by overflow or `+x / 0.0` for `x > 0`.
Arithmetic with infinity follows IEEE 754: finite + infinity is
infinity, infinity − infinity is NaN.

##### Examples

```
Float64.infinity;       // inf
Float64.infinity + 1;   // inf
1.0 / 0.0;              // inf
Float64.infinity.negate();  // -inf
```

_Defined in `lang/std/num/float32.ks`._

#### field `isFinite`

```kestrel
public var isFinite: Bool { get }
```

True if `self` is finite — equivalently, not NaN and not infinite.
Includes zero and subnormals.

_Defined in `lang/std/num/float32.ks`._

#### field `isInfinite`

```kestrel
public var isInfinite: Bool { get }
```

True if `self` is `+infinity` or `-infinity`.

##### Examples

```
Float64.infinity.isInfinite;             // true
Float64.infinity.negate().isInfinite;    // true
(1.0 / 0.0).isInfinite;                  // true
Float64.nan.isInfinite;                  // false
```

_Defined in `lang/std/num/float32.ks`._

#### field `isNaN`

```kestrel
public var isNaN: Bool { get }
```

True if `self` is NaN. The only correct way to test for NaN — `==`
returns false against NaN even when both operands are NaN.

##### Examples

```
(0.0 / 0.0).isNaN;        // true
Float64.nan.isNaN;        // true
(1.0).isNaN;              // false
Float64.infinity.isNaN;   // false
```

_Defined in `lang/std/num/float32.ks`._

#### field `isNegative`

```kestrel
public var isNegative: Bool { get }
```

True if `self < 0.0`. False for `-0.0`, `nan`, zero, and positives.
To detect signed zero specifically, use `sign`.

_Defined in `lang/std/num/float32.ks`._

#### field `isNormal`

```kestrel
public var isNormal: Bool { get }
```

True if `self` is a *normal* number — finite, non-zero, and at least
`minPositive` in magnitude. Subnormals, zero, infinity, and NaN are
not normal.

##### Examples

```
(1.0).isNormal;                              // true
(0.0).isNormal;                              // false
Float64.minPositive.isNormal;                // true
(Float64.minPositive / 2.0).isNormal;        // false (subnormal)
```

_Defined in `lang/std/num/float32.ks`._

#### field `isPositive`

```kestrel
public var isPositive: Bool { get }
```

True if `self > 0.0`. False for `+0.0`, `-0.0`, `nan`, and negatives.

_Defined in `lang/std/num/float32.ks`._

#### field `isSubnormal`

```kestrel
public var isSubnormal: Bool { get }
```

True if `self` is subnormal (denormalized) — finite, non-zero, and
smaller than `minPositive` in magnitude. Subnormals trade precision
for range near zero.

_Defined in `lang/std/num/float32.ks`._

#### field `isZero`

```kestrel
public var isZero: Bool { get }
```

True if `self` is `+0.0` or `-0.0`. Both signed zeros compare equal.

_Defined in `lang/std/num/float32.ks`._

#### function `lerp`

```kestrel
public func lerp(to: Float32, Float32) -> Float32
```

Linear interpolation — `self + (other - self) * t`. `t == 0` returns
`self`, `t == 1` returns `other`; `t` outside `[0, 1]` extrapolates.

##### Examples

```
(0.0).lerp(to: 10.0, t: 0.0);   //  0.0
(0.0).lerp(to: 10.0, t: 0.5);   //  5.0
(0.0).lerp(to: 10.0, t: 1.0);   // 10.0
(0.0).lerp(to: 10.0, t: 0.25);  //  2.5
```

_Defined in `lang/std/num/float32.ks`._

#### function `ln`

```kestrel
public func ln() -> Float32
```

Natural logarithm. Negatives return NaN; zero returns `-inf`.

##### Examples

```
(1.0).ln();           //  0.0
Float64.e.ln();       //  1.0
(0.0).ln();           // -inf
(-1.0).ln();          //  nan
```

_Defined in `lang/std/num/float32.ks`._

#### field `ln10`

```kestrel
public static var ln10: Float32 { get }
```

Natural logarithm of 10, ≈ 2.30258509299404…

_Defined in `lang/std/num/float32.ks`._

#### function `ln1p`

```kestrel
public func ln1p() -> Float32
```

`ln(1 + self)`, accurate for small `self` where `(1.0 + self).ln()`
would lose digits.

_Defined in `lang/std/num/float32.ks`._

#### field `ln2`

```kestrel
public static var ln2: Float32 { get }
```

Natural logarithm of 2, ≈ 0.69314718055994…

_Defined in `lang/std/num/float32.ks`._

#### function `log`

```kestrel
public func log(Float32) -> Float32
```

Logarithm with arbitrary base, computed as `self.ln() / base.ln()`.
Pick `log2` or `log10` directly when you can — they avoid the second
libm call.

_Defined in `lang/std/num/float32.ks`._

#### function `log10`

```kestrel
public func log10() -> Float32
```

Base-10 logarithm.

_Defined in `lang/std/num/float32.ks`._

#### function `log2`

```kestrel
public func log2() -> Float32
```

Base-2 logarithm.

_Defined in `lang/std/num/float32.ks`._

#### field `maxValue`

```kestrel
public static var maxValue: Float32 { get }
```

The most positive finite value, ≈ 3.4028235e38.

_Defined in `lang/std/num/float32.ks`._

#### field `minPositive`

```kestrel
public static var minPositive: Float32 { get }
```

The smallest positive *normal* value, ≈ 1.17549435e-38.
Values smaller than this are subnormal and lose precision.

_Defined in `lang/std/num/float32.ks`._

#### field `minValue`

```kestrel
public static var minValue: Float32 { get }
```

The most negative finite value, ≈ -3.4028235e38.

_Defined in `lang/std/num/float32.ks`._

#### field `nan`

```kestrel
public static var nan: Float32 { get }
```

Not-a-Number. Produced by undefined operations like `0.0 / 0.0` or
`sqrt(-1.0)`. NaN propagates through arithmetic and is unequal to
every value including itself — always test with `isNaN`.

##### Examples

```
Float64.nan.isNaN;             // true
Float64.nan == Float64.nan;    // false (!)
0.0 / 0.0;                     // nan
```

_Defined in `lang/std/num/float32.ks`._

#### function `nextDown`

```kestrel
public func nextDown() -> Float32
```

Next representable value less than `self`. Mirror of `nextUp`.

_Defined in `lang/std/num/float32.ks`._

#### function `nextUp`

```kestrel
public func nextUp() -> Float32
```

Next representable value greater than `self`. `+inf` and `nan` are
fixed points; the largest finite value steps up to `+inf`.

_Defined in `lang/std/num/float32.ks`._

#### function `parse`

```kestrel
public static func parse(String) -> Float32?
```

Parses a `Float32` from a string. Recognises decimal
(`"3.14"`), scientific (`"1.5e10"`, `"2.5E-3"`), and the special
tokens `"inf"`, `"-inf"`, `"+inf"`, `"infinity"`, `"nan"`
(case-insensitive). Returns `None` for any other input.

##### Examples

```
Float32.parse(string: "3.14");      // Some(3.14)
Float32.parse(string: "-2.5e10");   // Some(-2.5e10)
Float32.parse(string: "inf");       // Some(infinity)
Float32.parse(string: "nan");       // Some(nan)
Float32.parse(string: "abc");       // None
Float32.parse(string: "");          // None
```

_Defined in `lang/std/num/float32.ks`._

#### field `pi`

```kestrel
public static var pi: Float32 { get }
```

The constant π ≈ 3.14159265358979… — circle circumference over diameter.

_Defined in `lang/std/num/float32.ks`._

#### function `pow`

```kestrel
public func pow(Float32) -> Float32
```

`self ^ exponent` via libm. Negative bases with non-integer exponents
return NaN.

##### Examples

```
(2.0).pow(exponent: 10.0);   // 1024.0
(2.0).pow(exponent: 0.5);    // sqrt(2)
(-2.0).pow(exponent: 0.5);   // nan
```

_Defined in `lang/std/num/float32.ks`._

#### function `powi`

```kestrel
public func powi(Int64) -> Float32
```

Integer-exponent power via repeated squaring. Faster and more accurate
than `pow` when the exponent is known to be integral. Negative
exponents invert.

##### Examples

```
(2.0).powi(exponent: 10);   // 1024.0
(2.0).powi(exponent: -1);   // 0.5
```

_Defined in `lang/std/num/float32.ks`._

#### field `raw`

```kestrel
public var raw: lang.f32
```

The underlying primitive `lang.f32` value (IEEE 754 bit
pattern). Exposed for FFI and intrinsic use; reach for the typed
surface for everything else.

_Defined in `lang/std/num/float32.ks`._

#### function `remainder`

```kestrel
public func remainder(dividingBy: Float32) -> Float32
```

IEEE 754 remainder — uses round-to-nearest division, not truncation.
Differs from `%`: `(5.0).remainder(dividingBy: 3.0)` is `-1.0`, not
`2.0`.

_Defined in `lang/std/num/float32.ks`._

#### function `round`

```kestrel
public func round() -> Float32
```

Round to nearest integer, breaking ties *away from zero* (banker's
rounding is not used).

##### Examples

```
(3.4).round();   //  3.0
(3.5).round();   //  4.0   (tie → away from zero)
(-3.5).round();  // -4.0
```

_Defined in `lang/std/num/float32.ks`._

#### field `sign`

```kestrel
public var sign: Float32 { get }
```

Sign as a float: `-1.0`, `0.0`, or `1.0`. NaN propagates as NaN.
Negative zero returns `-0.0` (which still compares equal to `0.0`).

##### Examples

```
(-3.14).sign;        // -1.0
(0.0).sign;          //  0.0
(3.14).sign;         //  1.0
Float64.nan.sign;    //  nan
```

_Defined in `lang/std/num/float32.ks`._

#### function `sin`

```kestrel
public func sin() -> Float32
```

Sine of `self` in radians.

_Defined in `lang/std/num/float32.ks`._

#### function `sinCos`

```kestrel
public func sinCos() -> (Float32, Float32)
```

Sine and cosine in one call. Implemented via two libm calls today;
kept for ergonomics and as a future optimisation point.

##### Examples

```
let (s, c) = angle.sinCos();
```

_Defined in `lang/std/num/float32.ks`._

#### function `sinh`

```kestrel
public func sinh() -> Float32
```

Hyperbolic sine.

_Defined in `lang/std/num/float32.ks`._

#### function `sqrt`

```kestrel
public func sqrt() -> Float32
```

Principal square root. Negatives return NaN (`-0.0` returns `-0.0`);
`+inf` returns `+inf`.

##### Examples

```
(4.0).sqrt();    // 2.0
(-1.0).sqrt();   // nan
```

_Defined in `lang/std/num/float32.ks`._

#### field `sqrt2`

```kestrel
public static var sqrt2: Float32 { get }
```

Square root of 2, ≈ 1.41421356237309…

_Defined in `lang/std/num/float32.ks`._

#### function `tan`

```kestrel
public func tan() -> Float32
```

Tangent of `self` in radians. Diverges to ±large near `π/2 + kπ`.

_Defined in `lang/std/num/float32.ks`._

#### function `tanh`

```kestrel
public func tanh() -> Float32
```

Hyperbolic tangent. Saturates at ±1 for large magnitudes.

_Defined in `lang/std/num/float32.ks`._

#### field `tau`

```kestrel
public static var tau: Float32 { get }
```

Tau ≈ 6.28318530717958… — equal to `2π`, often more natural for
"one full turn" rotational math.

_Defined in `lang/std/num/float32.ks`._

#### function `toFloat64`

```kestrel
public func toFloat64() -> Float64
```

Converts to the sibling float type. Widening (32→64) is exact;
narrowing (64→32) rounds and may overflow to ±infinity.

_Defined in `lang/std/num/float32.ks`._

#### function `toInt64`

```kestrel
public func toInt64() -> Int64?
```

Truncates toward zero into an `Int64`. Returns `None` for NaN,
infinity, or values that fall outside the `Int64` range.

##### Examples

```
(3.7).toInt64();              // Some(3)
(-3.7).toInt64();             // Some(-3)
Float64.nan.toInt64();        // None
Float64.infinity.toInt64();   // None
```

_Defined in `lang/std/num/float32.ks`._

#### function `trunc`

```kestrel
public func trunc() -> Float32
```

Integer part, truncating toward zero. `floor` for positives, `ceil`
for negatives.

_Defined in `lang/std/num/float32.ks`._

### Implements `Comparable`

#### function `compare`

```kestrel
public func compare(Float32) -> Ordering
```

Three-way comparison returning an `Ordering`. NaN is *not* an ordered
value — comparisons against NaN currently fall through to `.Equal`,
which is wrong; gate inputs with `isNaN` if you need a well-defined
answer.

##### Examples

```
(1.0).compare(other: 2.0);              // .Less
(2.0).compare(other: 2.0);              // .Equal
(3.0).compare(other: 2.0);              // .Greater
(1.0).compare(other: Float64.infinity); // .Less
```

_Defined in `lang/std/num/float32.ks`._

### Implements `Equatable`

#### function `equals`

```kestrel
public func equals(Float32) -> Bool
```

IEEE 754 equality. NaN is not equal to itself; `+0.0` equals `-0.0`.

##### Examples

```
(3.14).equals(other: 3.14);                  // true
(0.0).equals(other: -0.0);                   // true
Float64.nan.equals(other: Float64.nan);      // false (!)
```

_Defined in `lang/std/num/float32.ks`._

### Implements `Formattable`

#### function `format`

```kestrel
public func format(FormatOptions) -> String
```

Renders the float to a `String`, honouring the supplied
`FormatOptions`. Implements `Formattable`.

Recognised options:
- `precision` — digits after the decimal point (default 6).
- `width` / `fill` / `alignment` — padding control.
- `sign` — `.Negative` (default), `.Always`, or `.Space`.
- `floatStyle` — `.Fixed`, `.Scientific`, `.Auto`, or `.Percent`.
  `.Auto` picks fixed or scientific based on magnitude.
  `.Percent` multiplies by 100 and appends `%`.

String interpolation forwards through the same options:
`"\{x:.2}"` is two decimal places, `"\{x:.2e}"` is scientific,
`"\{x:%}"` is percentage.

##### Examples

```
(3.14159).format();                                          // "3.14159"
(3.14159).format(options: .{precision: 2});                  // "3.14"
(1234.5).format(options: .{floatStyle: .Scientific});        // "1.2345e3"
(0.756).format(options: .{floatStyle: .Percent});            // "75.6%"
(3.14).format(options: .{width: 8, fill: '0'});              // "00003.14"
(3.14).format(options: .{sign: .Always});                    // "+3.14"
```

_Defined in `lang/std/num/float32.ks`._

### Implements `Addable`

#### typealias `Output`

```kestrel
type Output = Float32
```

_Defined in `lang/std/num/float32.ks`._

#### typealias `Output`

```kestrel
type Output = Float32
```

_Defined in `lang/std/num/float32.ks`._

#### typealias `Output`

```kestrel
type Output = Float32
```

_Defined in `lang/std/num/float32.ks`._

#### typealias `Output`

```kestrel
type Output = Float32
```

_Defined in `lang/std/num/float32.ks`._

#### typealias `Output`

```kestrel
type Output = Float32
```

_Defined in `lang/std/num/float32.ks`._

#### function `add`

```kestrel
public func add(Float32) -> Float32
```

IEEE 754 addition. NaN propagates; `inf + (-inf)` is NaN; finite + inf
is inf.

_Defined in `lang/std/num/float32.ks`._

#### field `zero`

```kestrel
public static var zero: Float32 { get }
```

The additive identity, `0.0`.

_Defined in `lang/std/num/float32.ks`._

### Implements `Subtractable`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/arithmetic.ks`._

#### function `subtract`

```kestrel
public func subtract(Float32) -> Float32
```

IEEE 754 subtraction. `inf - inf` is NaN; otherwise mirrors `add`.

_Defined in `lang/std/num/float32.ks`._

### Implements `Multipliable`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/arithmetic.ks`._

#### function `multiply`

```kestrel
public func multiply(Float32) -> Float32
```

IEEE 754 multiplication. NaN propagates; `inf * 0` is NaN; sign of
the result follows the usual algebra.

_Defined in `lang/std/num/float32.ks`._

#### field `one`

```kestrel
public static var one: Float32 { get }
```

The multiplicative identity, `1.0`.

_Defined in `lang/std/num/float32.ks`._

### Implements `Divisible`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/arithmetic.ks`._

#### function `divide`

```kestrel
public func divide(Float32) -> Float32
```

IEEE 754 division. Unlike integer divide, dividing by zero does not
trap — it produces ±infinity (or NaN for `0.0 / 0.0`).

Special cases:
- `x / 0.0` → `+inf` if `x > 0`, `-inf` if `x < 0`, `nan` if `x == 0`
- `x / inf` → 0 for finite `x`
- `inf / inf` → NaN

##### Examples

```
(10.0).divide(other: 4.0);  // 2.5
1.0 / 0.0;                  // inf
0.0 / 0.0;                  // nan
```

_Defined in `lang/std/num/float32.ks`._

### Implements `Negatable`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/arithmetic.ks`._

#### function `negate`

```kestrel
public func negate() -> Float32
```

IEEE 754 negation — flips the sign bit. `-nan` is still NaN; `-(-0.0)`
is `+0.0`.

_Defined in `lang/std/num/float32.ks`._

### Implements `ExpressibleByFloatLiteral`

#### initializer `Float Literal`

```kestrel
init(floatLiteral: lang.f64)
```

Builds an instance from a floating-point literal.

_Defined in `lang/std/core/literals.ks`._

### Implements `ExpressibleByIntLiteral`

#### initializer `Int Literal`

```kestrel
init(intLiteral: lang.i64)
```

Builds an instance from an integer literal.

_Defined in `lang/std/core/literals.ks`._

### Implements `Defaultable`

#### initializer `Default`

```kestrel
init()
```

Builds the default-valued instance.

_Defined in `lang/std/core/protocols.ks`._

### Implements `Convertible`

#### initializer `From Source`

```kestrel
init(from: From)
```

Creates an instance from `value`.

_Defined in `lang/std/core/convertible.ks`._

## struct `Float64`

```kestrel
public struct Float64 { /* private fields */ }
```

A 64-bit IEEE 754 double-precision float.

Range is approximately ±1.8×10^308 with 15-17 significant decimal
digits. Float literals without a type annotation default to `Float64`;
annotate the binding to pick `Float32`. The type is `FFISafe` and lays out
as a single `lang.f64`.

### Examples

```
let pi = Float64.pi;
let area = pi * radius * radius;
let s = area.format(options: .{precision: 2});  // "314.16"
```

```
let x = 3.14;          // Float64
let y: Float32 = 3.14; // Float32
```

### Special Values

- `nan` — Not-a-Number, result of `0.0 / 0.0`, `sqrt(-1)`, etc.
- `infinity` / `-infinity` — overflow or `1.0 / 0.0`.
- Negative zero compares equal to positive zero but produces `-infinity`
  when used as a divisor.

NaN comparisons are surprising: `nan == nan` is false and every ordered
comparison against NaN is false. Use `isNaN` to test, never `== nan`. Any
arithmetic with NaN propagates NaN.

### Representation

A single `lang.f64` field holding the raw IEEE 754 bit pattern.

_Defined in `lang/std/num/float64.ks`._

### Members

#### initializer `Default`

```kestrel
public init()
```

Creates the zero value, satisfying `Defaultable`.

_Defined in `lang/std/num/float64.ks`._

#### initializer `Float Literal`

```kestrel
public init(floatLiteral: lang.f64)
```

Compiler-emitted bridge for floating-point literals via
`ExpressibleByFloatLiteral`. Rarely called directly.

##### Examples

```
let x: Float64 = 3.14;                  // implicit
let y = Float64(floatLiteral: 3.14);    // explicit
```

_Defined in `lang/std/num/float64.ks`._

#### initializer `From Float`

```kestrel
public init(from: Float32)
```

Converts between `Float32` and `Float64`. The 32→64 direction is
exact; the 64→32 direction rounds and may overflow to ±infinity.

##### Examples

```
let f32: Float32 = 3.14;
let f64 = Float64(from: f32);
```

_Defined in `lang/std/num/float64.ks`._

#### initializer `From Int`

```kestrel
public init(from: Int64)
```

Converts an `Int64` to a float. Values with magnitude greater than
2^53 lose low-order bits.

##### Examples

```
let n: Int64 = 42;
let f = Float64(from: n);  // 42.0
```

_Defined in `lang/std/num/float64.ks`._

#### initializer `From Raw`

```kestrel
init(raw: lang.f64)
```

Wraps an existing `lang.f64` bit pattern. Internal; used
by intrinsics.

_Defined in `lang/std/num/float64.ks`._

#### initializer `Int Literal`

```kestrel
public init(intLiteral: lang.i64)
```

Bridge that lets bare integer literals appear where a float is
expected. Conversion is exact up to ±2^53; larger magnitudes round.

##### Examples

```
let x: Float64 = 42;     // 42.0
let y = 3.14 + 1;        // 4.14 — `1` widened to Float64
```

_Defined in `lang/std/num/float64.ks`._

#### function `abs`

```kestrel
public func abs() -> Float64
```

Absolute value — clears the sign bit. NaN stays NaN; `-0.0` becomes
`+0.0`.

_Defined in `lang/std/num/float64.ks`._

#### function `acos`

```kestrel
public func acos() -> Float64
```

Arc cosine, result in radians on `[0, π]`. Returns NaN outside
`[-1.0, 1.0]`.

_Defined in `lang/std/num/float64.ks`._

#### function `acosh`

```kestrel
public func acosh() -> Float64
```

Inverse hyperbolic cosine. Returns NaN for inputs less than `1.0`.

_Defined in `lang/std/num/float64.ks`._

#### function `asin`

```kestrel
public func asin() -> Float64
```

Arc sine, result in radians on `[-π/2, π/2]`. Returns NaN outside
`[-1.0, 1.0]`.

_Defined in `lang/std/num/float64.ks`._

#### function `asinh`

```kestrel
public func asinh() -> Float64
```

Inverse hyperbolic sine. Defined on all real inputs.

_Defined in `lang/std/num/float64.ks`._

#### function `atan`

```kestrel
public func atan() -> Float64
```

Arc tangent, result in radians on `[-π/2, π/2]`. For full-quadrant
recovery use `atan2`.

_Defined in `lang/std/num/float64.ks`._

#### function `atan2`

```kestrel
public func atan2(Float64) -> Float64
```

Two-argument arctangent — angle of the point `(x, self)` measured
from the positive x-axis, on `[-π, π]`. Disambiguates quadrant where
`atan` cannot.

##### Examples

```
(1.0).atan2(x: 1.0);    //  π/4   (Q1)
(1.0).atan2(x: -1.0);   //  3π/4  (Q2)
(-1.0).atan2(x: -1.0);  // -3π/4  (Q3)
(-1.0).atan2(x: 1.0);   // -π/4   (Q4)
```

_Defined in `lang/std/num/float64.ks`._

#### function `atanh`

```kestrel
public func atanh() -> Float64
```

Inverse hyperbolic tangent. NaN outside `(-1.0, 1.0)`; `±inf` at ±1.

_Defined in `lang/std/num/float64.ks`._

#### function `cbrt`

```kestrel
public func cbrt() -> Float64
```

Real cube root. Defined for negatives — `(-8.0).cbrt() == -2.0`.

_Defined in `lang/std/num/float64.ks`._

#### function `ceil`

```kestrel
public func ceil() -> Float64
```

Smallest integer ≥ `self`. Rounds toward `+infinity`.

##### Examples

```
(3.2).ceil();   //  4.0
(-3.7).ceil();  // -3.0
```

_Defined in `lang/std/num/float64.ks`._

#### function `clamp`

```kestrel
public func clamp(Float64, Float64) -> Float64
```

Clamps `self` into `[min, max]`. NaN passes through unchanged. Caller
must ensure `min <= max`.

##### Examples

```
(0.5).clamp(min: 0.0, max: 1.0);   // 0.5
(-0.5).clamp(min: 0.0, max: 1.0);  // 0.0
(1.5).clamp(min: 0.0, max: 1.0);   // 1.0
```

_Defined in `lang/std/num/float64.ks`._

#### function `copysign`

```kestrel
public func copysign(from: Float64) -> Float64
```

Returns a value with `self`'s magnitude and `other`'s sign — i.e. an
IEEE 754 `copysign`. Useful for unbiased rounding tricks.

_Defined in `lang/std/num/float64.ks`._

#### function `cos`

```kestrel
public func cos() -> Float64
```

Cosine of `self` in radians.

_Defined in `lang/std/num/float64.ks`._

#### function `cosh`

```kestrel
public func cosh() -> Float64
```

Hyperbolic cosine.

_Defined in `lang/std/num/float64.ks`._

#### field `e`

```kestrel
public static var e: Float64 { get }
```

Euler's number `e` ≈ 2.71828182845904… — base of the natural logarithm.

_Defined in `lang/std/num/float64.ks`._

#### field `epsilon`

```kestrel
public static var epsilon: Float64 { get }
```

Machine epsilon — the smallest `e` such that `1.0 + e != 1.0`,
≈ 2.220446049250313e-16.

Useful as a tolerance in approximate comparisons; scale by the
operand magnitude for relative-error checks.

##### Examples

```
func almostEqual(a: Float64, b: Float64) -> Bool {
    (a - b).abs() < Float64.epsilon * a.abs().max(b.abs());
}
```

_Defined in `lang/std/num/float64.ks`._

#### function `exp`

```kestrel
public func exp() -> Float64
```

`e^self` via libm. `(-inf).exp()` is `0.0`; `(inf).exp()` is `inf`.

_Defined in `lang/std/num/float64.ks`._

#### function `exp2`

```kestrel
public func exp2() -> Float64
```

`2^self`. Useful for binary scaling.

_Defined in `lang/std/num/float64.ks`._

#### function `expm1`

```kestrel
public func expm1() -> Float64
```

`e^self - 1`, computed without the cancellation that hurts
`self.exp() - 1.0` for small `self`.

_Defined in `lang/std/num/float64.ks`._

#### function `floor`

```kestrel
public func floor() -> Float64
```

Largest integer ≤ `self`. Rounds toward `-infinity`.

##### Examples

```
(3.7).floor();   //  3.0
(-3.2).floor();  // -4.0
```

_Defined in `lang/std/num/float64.ks`._

#### function `fma`

```kestrel
public func fma(Float64, Float64) -> Float64
```

Fused multiply-add — `(self * a) + b` with a single rounding step.
More accurate (and often faster) than separate `multiply`/`add`.

##### Examples

```
(2.0).fma(a: 3.0, b: 4.0);   // 10.0
```

_Defined in `lang/std/num/float64.ks`._

#### function `fract`

```kestrel
public func fract() -> Float64
```

Fractional part — `self - self.trunc()`. Sign matches `self`.

##### Examples

```
(3.7).fract();   //  0.7
(-3.7).fract();  // -0.7
```

_Defined in `lang/std/num/float64.ks`._

#### function `hypot`

```kestrel
public func hypot(Float64) -> Float64
```

Hypotenuse — `sqrt(self² + other²)`, computed via libm in a way that
avoids intermediate overflow when one operand is very large.

##### Examples

```
(3.0).hypot(other: 4.0);  // 5.0
```

_Defined in `lang/std/num/float64.ks`._

#### field `infinity`

```kestrel
public static var infinity: Float64 { get }
```

Positive infinity. Produced by overflow or `+x / 0.0` for `x > 0`.
Arithmetic with infinity follows IEEE 754: finite + infinity is
infinity, infinity − infinity is NaN.

##### Examples

```
Float64.infinity;       // inf
Float64.infinity + 1;   // inf
1.0 / 0.0;              // inf
Float64.infinity.negate();  // -inf
```

_Defined in `lang/std/num/float64.ks`._

#### field `isFinite`

```kestrel
public var isFinite: Bool { get }
```

True if `self` is finite — equivalently, not NaN and not infinite.
Includes zero and subnormals.

_Defined in `lang/std/num/float64.ks`._

#### field `isInfinite`

```kestrel
public var isInfinite: Bool { get }
```

True if `self` is `+infinity` or `-infinity`.

##### Examples

```
Float64.infinity.isInfinite;             // true
Float64.infinity.negate().isInfinite;    // true
(1.0 / 0.0).isInfinite;                  // true
Float64.nan.isInfinite;                  // false
```

_Defined in `lang/std/num/float64.ks`._

#### field `isNaN`

```kestrel
public var isNaN: Bool { get }
```

True if `self` is NaN. The only correct way to test for NaN — `==`
returns false against NaN even when both operands are NaN.

##### Examples

```
(0.0 / 0.0).isNaN;        // true
Float64.nan.isNaN;        // true
(1.0).isNaN;              // false
Float64.infinity.isNaN;   // false
```

_Defined in `lang/std/num/float64.ks`._

#### field `isNegative`

```kestrel
public var isNegative: Bool { get }
```

True if `self < 0.0`. False for `-0.0`, `nan`, zero, and positives.
To detect signed zero specifically, use `sign`.

_Defined in `lang/std/num/float64.ks`._

#### field `isNormal`

```kestrel
public var isNormal: Bool { get }
```

True if `self` is a *normal* number — finite, non-zero, and at least
`minPositive` in magnitude. Subnormals, zero, infinity, and NaN are
not normal.

##### Examples

```
(1.0).isNormal;                              // true
(0.0).isNormal;                              // false
Float64.minPositive.isNormal;                // true
(Float64.minPositive / 2.0).isNormal;        // false (subnormal)
```

_Defined in `lang/std/num/float64.ks`._

#### field `isPositive`

```kestrel
public var isPositive: Bool { get }
```

True if `self > 0.0`. False for `+0.0`, `-0.0`, `nan`, and negatives.

_Defined in `lang/std/num/float64.ks`._

#### field `isSubnormal`

```kestrel
public var isSubnormal: Bool { get }
```

True if `self` is subnormal (denormalized) — finite, non-zero, and
smaller than `minPositive` in magnitude. Subnormals trade precision
for range near zero.

_Defined in `lang/std/num/float64.ks`._

#### field `isZero`

```kestrel
public var isZero: Bool { get }
```

True if `self` is `+0.0` or `-0.0`. Both signed zeros compare equal.

_Defined in `lang/std/num/float64.ks`._

#### function `lerp`

```kestrel
public func lerp(to: Float64, Float64) -> Float64
```

Linear interpolation — `self + (other - self) * t`. `t == 0` returns
`self`, `t == 1` returns `other`; `t` outside `[0, 1]` extrapolates.

##### Examples

```
(0.0).lerp(to: 10.0, t: 0.0);   //  0.0
(0.0).lerp(to: 10.0, t: 0.5);   //  5.0
(0.0).lerp(to: 10.0, t: 1.0);   // 10.0
(0.0).lerp(to: 10.0, t: 0.25);  //  2.5
```

_Defined in `lang/std/num/float64.ks`._

#### function `ln`

```kestrel
public func ln() -> Float64
```

Natural logarithm. Negatives return NaN; zero returns `-inf`.

##### Examples

```
(1.0).ln();           //  0.0
Float64.e.ln();       //  1.0
(0.0).ln();           // -inf
(-1.0).ln();          //  nan
```

_Defined in `lang/std/num/float64.ks`._

#### field `ln10`

```kestrel
public static var ln10: Float64 { get }
```

Natural logarithm of 10, ≈ 2.30258509299404…

_Defined in `lang/std/num/float64.ks`._

#### function `ln1p`

```kestrel
public func ln1p() -> Float64
```

`ln(1 + self)`, accurate for small `self` where `(1.0 + self).ln()`
would lose digits.

_Defined in `lang/std/num/float64.ks`._

#### field `ln2`

```kestrel
public static var ln2: Float64 { get }
```

Natural logarithm of 2, ≈ 0.69314718055994…

_Defined in `lang/std/num/float64.ks`._

#### function `log`

```kestrel
public func log(Float64) -> Float64
```

Logarithm with arbitrary base, computed as `self.ln() / base.ln()`.
Pick `log2` or `log10` directly when you can — they avoid the second
libm call.

_Defined in `lang/std/num/float64.ks`._

#### function `log10`

```kestrel
public func log10() -> Float64
```

Base-10 logarithm.

_Defined in `lang/std/num/float64.ks`._

#### function `log2`

```kestrel
public func log2() -> Float64
```

Base-2 logarithm.

_Defined in `lang/std/num/float64.ks`._

#### field `maxValue`

```kestrel
public static var maxValue: Float64 { get }
```

The most positive finite value, ≈ 1.7976931348623157e308.

_Defined in `lang/std/num/float64.ks`._

#### field `minPositive`

```kestrel
public static var minPositive: Float64 { get }
```

The smallest positive *normal* value, ≈ 2.2250738585072014e-308.
Values smaller than this are subnormal and lose precision.

_Defined in `lang/std/num/float64.ks`._

#### field `minValue`

```kestrel
public static var minValue: Float64 { get }
```

The most negative finite value, ≈ -1.7976931348623157e308.

_Defined in `lang/std/num/float64.ks`._

#### field `nan`

```kestrel
public static var nan: Float64 { get }
```

Not-a-Number. Produced by undefined operations like `0.0 / 0.0` or
`sqrt(-1.0)`. NaN propagates through arithmetic and is unequal to
every value including itself — always test with `isNaN`.

##### Examples

```
Float64.nan.isNaN;             // true
Float64.nan == Float64.nan;    // false (!)
0.0 / 0.0;                     // nan
```

_Defined in `lang/std/num/float64.ks`._

#### function `nextDown`

```kestrel
public func nextDown() -> Float64
```

Next representable value less than `self`. Mirror of `nextUp`.

_Defined in `lang/std/num/float64.ks`._

#### function `nextUp`

```kestrel
public func nextUp() -> Float64
```

Next representable value greater than `self`. `+inf` and `nan` are
fixed points; the largest finite value steps up to `+inf`.

_Defined in `lang/std/num/float64.ks`._

#### function `parse`

```kestrel
public static func parse(String) -> Float64?
```

Parses a `Float64` from a string. Recognises decimal
(`"3.14"`), scientific (`"1.5e10"`, `"2.5E-3"`), and the special
tokens `"inf"`, `"-inf"`, `"+inf"`, `"infinity"`, `"nan"`
(case-insensitive). Returns `None` for any other input.

##### Examples

```
Float64.parse(string: "3.14");      // Some(3.14)
Float64.parse(string: "-2.5e10");   // Some(-2.5e10)
Float64.parse(string: "inf");       // Some(infinity)
Float64.parse(string: "nan");       // Some(nan)
Float64.parse(string: "abc");       // None
Float64.parse(string: "");          // None
```

_Defined in `lang/std/num/float64.ks`._

#### field `pi`

```kestrel
public static var pi: Float64 { get }
```

The constant π ≈ 3.14159265358979… — circle circumference over diameter.

_Defined in `lang/std/num/float64.ks`._

#### function `pow`

```kestrel
public func pow(Float64) -> Float64
```

`self ^ exponent` via libm. Negative bases with non-integer exponents
return NaN.

##### Examples

```
(2.0).pow(exponent: 10.0);   // 1024.0
(2.0).pow(exponent: 0.5);    // sqrt(2)
(-2.0).pow(exponent: 0.5);   // nan
```

_Defined in `lang/std/num/float64.ks`._

#### function `powi`

```kestrel
public func powi(Int64) -> Float64
```

Integer-exponent power via repeated squaring. Faster and more accurate
than `pow` when the exponent is known to be integral. Negative
exponents invert.

##### Examples

```
(2.0).powi(exponent: 10);   // 1024.0
(2.0).powi(exponent: -1);   // 0.5
```

_Defined in `lang/std/num/float64.ks`._

#### field `raw`

```kestrel
public var raw: lang.f64
```

The underlying primitive `lang.f64` value (IEEE 754 bit
pattern). Exposed for FFI and intrinsic use; reach for the typed
surface for everything else.

_Defined in `lang/std/num/float64.ks`._

#### function `remainder`

```kestrel
public func remainder(dividingBy: Float64) -> Float64
```

IEEE 754 remainder — uses round-to-nearest division, not truncation.
Differs from `%`: `(5.0).remainder(dividingBy: 3.0)` is `-1.0`, not
`2.0`.

_Defined in `lang/std/num/float64.ks`._

#### function `round`

```kestrel
public func round() -> Float64
```

Round to nearest integer, breaking ties *away from zero* (banker's
rounding is not used).

##### Examples

```
(3.4).round();   //  3.0
(3.5).round();   //  4.0   (tie → away from zero)
(-3.5).round();  // -4.0
```

_Defined in `lang/std/num/float64.ks`._

#### field `sign`

```kestrel
public var sign: Float64 { get }
```

Sign as a float: `-1.0`, `0.0`, or `1.0`. NaN propagates as NaN.
Negative zero returns `-0.0` (which still compares equal to `0.0`).

##### Examples

```
(-3.14).sign;        // -1.0
(0.0).sign;          //  0.0
(3.14).sign;         //  1.0
Float64.nan.sign;    //  nan
```

_Defined in `lang/std/num/float64.ks`._

#### function `sin`

```kestrel
public func sin() -> Float64
```

Sine of `self` in radians.

_Defined in `lang/std/num/float64.ks`._

#### function `sinCos`

```kestrel
public func sinCos() -> (Float64, Float64)
```

Sine and cosine in one call. Implemented via two libm calls today;
kept for ergonomics and as a future optimisation point.

##### Examples

```
let (s, c) = angle.sinCos();
```

_Defined in `lang/std/num/float64.ks`._

#### function `sinh`

```kestrel
public func sinh() -> Float64
```

Hyperbolic sine.

_Defined in `lang/std/num/float64.ks`._

#### function `sqrt`

```kestrel
public func sqrt() -> Float64
```

Principal square root. Negatives return NaN (`-0.0` returns `-0.0`);
`+inf` returns `+inf`.

##### Examples

```
(4.0).sqrt();    // 2.0
(-1.0).sqrt();   // nan
```

_Defined in `lang/std/num/float64.ks`._

#### field `sqrt2`

```kestrel
public static var sqrt2: Float64 { get }
```

Square root of 2, ≈ 1.41421356237309…

_Defined in `lang/std/num/float64.ks`._

#### function `tan`

```kestrel
public func tan() -> Float64
```

Tangent of `self` in radians. Diverges to ±large near `π/2 + kπ`.

_Defined in `lang/std/num/float64.ks`._

#### function `tanh`

```kestrel
public func tanh() -> Float64
```

Hyperbolic tangent. Saturates at ±1 for large magnitudes.

_Defined in `lang/std/num/float64.ks`._

#### field `tau`

```kestrel
public static var tau: Float64 { get }
```

Tau ≈ 6.28318530717958… — equal to `2π`, often more natural for
"one full turn" rotational math.

_Defined in `lang/std/num/float64.ks`._

#### function `toFloat32`

```kestrel
public func toFloat32() -> Float32
```

Converts to the sibling float type. Widening (32→64) is exact;
narrowing (64→32) rounds and may overflow to ±infinity.

_Defined in `lang/std/num/float64.ks`._

#### function `toInt64`

```kestrel
public func toInt64() -> Int64?
```

Truncates toward zero into an `Int64`. Returns `None` for NaN,
infinity, or values that fall outside the `Int64` range.

##### Examples

```
(3.7).toInt64();              // Some(3)
(-3.7).toInt64();             // Some(-3)
Float64.nan.toInt64();        // None
Float64.infinity.toInt64();   // None
```

_Defined in `lang/std/num/float64.ks`._

#### function `trunc`

```kestrel
public func trunc() -> Float64
```

Integer part, truncating toward zero. `floor` for positives, `ceil`
for negatives.

_Defined in `lang/std/num/float64.ks`._

### Implements `Comparable`

#### function `compare`

```kestrel
public func compare(Float64) -> Ordering
```

Three-way comparison returning an `Ordering`. NaN is *not* an ordered
value — comparisons against NaN currently fall through to `.Equal`,
which is wrong; gate inputs with `isNaN` if you need a well-defined
answer.

##### Examples

```
(1.0).compare(other: 2.0);              // .Less
(2.0).compare(other: 2.0);              // .Equal
(3.0).compare(other: 2.0);              // .Greater
(1.0).compare(other: Float64.infinity); // .Less
```

_Defined in `lang/std/num/float64.ks`._

### Implements `Equatable`

#### function `equals`

```kestrel
public func equals(Float64) -> Bool
```

IEEE 754 equality. NaN is not equal to itself; `+0.0` equals `-0.0`.

##### Examples

```
(3.14).equals(other: 3.14);                  // true
(0.0).equals(other: -0.0);                   // true
Float64.nan.equals(other: Float64.nan);      // false (!)
```

_Defined in `lang/std/num/float64.ks`._

### Implements `Formattable`

#### function `format`

```kestrel
public func format(FormatOptions) -> String
```

Renders the float to a `String`, honouring the supplied
`FormatOptions`. Implements `Formattable`.

Recognised options:
- `precision` — digits after the decimal point (default 6).
- `width` / `fill` / `alignment` — padding control.
- `sign` — `.Negative` (default), `.Always`, or `.Space`.
- `floatStyle` — `.Fixed`, `.Scientific`, `.Auto`, or `.Percent`.
  `.Auto` picks fixed or scientific based on magnitude.
  `.Percent` multiplies by 100 and appends `%`.

String interpolation forwards through the same options:
`"\{x:.2}"` is two decimal places, `"\{x:.2e}"` is scientific,
`"\{x:%}"` is percentage.

##### Examples

```
(3.14159).format();                                          // "3.14159"
(3.14159).format(options: .{precision: 2});                  // "3.14"
(1234.5).format(options: .{floatStyle: .Scientific});        // "1.2345e3"
(0.756).format(options: .{floatStyle: .Percent});            // "75.6%"
(3.14).format(options: .{width: 8, fill: '0'});              // "00003.14"
(3.14).format(options: .{sign: .Always});                    // "+3.14"
```

_Defined in `lang/std/num/float64.ks`._

### Implements `Addable`

#### typealias `Output`

```kestrel
type Output = Float64
```

_Defined in `lang/std/num/float64.ks`._

#### typealias `Output`

```kestrel
type Output = Float64
```

_Defined in `lang/std/num/float64.ks`._

#### typealias `Output`

```kestrel
type Output = Float64
```

_Defined in `lang/std/num/float64.ks`._

#### typealias `Output`

```kestrel
type Output = Float64
```

_Defined in `lang/std/num/float64.ks`._

#### typealias `Output`

```kestrel
type Output = Float64
```

_Defined in `lang/std/num/float64.ks`._

#### function `add`

```kestrel
public func add(Float64) -> Float64
```

IEEE 754 addition. NaN propagates; `inf + (-inf)` is NaN; finite + inf
is inf.

_Defined in `lang/std/num/float64.ks`._

#### field `zero`

```kestrel
public static var zero: Float64 { get }
```

The additive identity, `0.0`.

_Defined in `lang/std/num/float64.ks`._

### Implements `Subtractable`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/arithmetic.ks`._

#### function `subtract`

```kestrel
public func subtract(Float64) -> Float64
```

IEEE 754 subtraction. `inf - inf` is NaN; otherwise mirrors `add`.

_Defined in `lang/std/num/float64.ks`._

### Implements `Multipliable`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/arithmetic.ks`._

#### function `multiply`

```kestrel
public func multiply(Float64) -> Float64
```

IEEE 754 multiplication. NaN propagates; `inf * 0` is NaN; sign of
the result follows the usual algebra.

_Defined in `lang/std/num/float64.ks`._

#### field `one`

```kestrel
public static var one: Float64 { get }
```

The multiplicative identity, `1.0`.

_Defined in `lang/std/num/float64.ks`._

### Implements `Divisible`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/arithmetic.ks`._

#### function `divide`

```kestrel
public func divide(Float64) -> Float64
```

IEEE 754 division. Unlike integer divide, dividing by zero does not
trap — it produces ±infinity (or NaN for `0.0 / 0.0`).

Special cases:
- `x / 0.0` → `+inf` if `x > 0`, `-inf` if `x < 0`, `nan` if `x == 0`
- `x / inf` → 0 for finite `x`
- `inf / inf` → NaN

##### Examples

```
(10.0).divide(other: 4.0);  // 2.5
1.0 / 0.0;                  // inf
0.0 / 0.0;                  // nan
```

_Defined in `lang/std/num/float64.ks`._

### Implements `Negatable`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/arithmetic.ks`._

#### function `negate`

```kestrel
public func negate() -> Float64
```

IEEE 754 negation — flips the sign bit. `-nan` is still NaN; `-(-0.0)`
is `+0.0`.

_Defined in `lang/std/num/float64.ks`._

### Implements `ExpressibleByFloatLiteral`

#### initializer `Float Literal`

```kestrel
init(floatLiteral: lang.f64)
```

Builds an instance from a floating-point literal.

_Defined in `lang/std/core/literals.ks`._

### Implements `ExpressibleByIntLiteral`

#### initializer `Int Literal`

```kestrel
init(intLiteral: lang.i64)
```

Builds an instance from an integer literal.

_Defined in `lang/std/core/literals.ks`._

### Implements `Defaultable`

#### initializer `Default`

```kestrel
init()
```

Builds the default-valued instance.

_Defined in `lang/std/core/protocols.ks`._

### Implements `Convertible`

#### initializer `From Source`

```kestrel
init(from: From)
```

Creates an instance from `value`.

_Defined in `lang/std/core/convertible.ks`._

## typealias `Int`

```kestrel
public type Int = Int64
```

Platform-sized signed integer — currently always `Int64`.

_Defined in `lang/std/num/int64.ks`._

## struct `Int16`

```kestrel
public struct Int16 { /* private fields */ }
```

A 16-bit signed integer.

Int16 is the 16-bit member of the integer family. The same surface
area is provided across all widths; switch widths to trade range for memory
or to match an FFI ABI. Arithmetic wraps on overflow by default — use the
`*Checked` variants for overflow detection or `*Saturating` to clamp to
`minValue`/`maxValue`. The type is `FFISafe` and lays out as a single
`lang.i16` so it can cross C boundaries unchanged.

### Examples

```
let a: Int64 = 100;
let b = a + 50;        // 150
let c = a * 2;         // 200
let d = a.addChecked(Int64.maxValue);  // None (overflow detected)
```

```
// Bit twiddling
(0b1010).countOnes      // 2
(1).shiftLeft(by: 4)    // 16
(-1).leadingZeros       // 0  (all bits set)
```

### Representation

A single `lang.i16` field. No padding, no headers — bit-identical
to the corresponding C type.

_Defined in `lang/std/num/int16.ks`._

### Members

#### initializer `Default`

```kestrel
public init()
```

Creates the zero value, satisfying `Defaultable`.

##### Examples

```
let n = Int64();   // 0
```

_Defined in `lang/std/num/int16.ks`._

#### initializer `From Integer`

```kestrel
public init(from: Int8)
```

Converts from `Int8`. Narrowing conversions truncate the high
bits; signed→unsigned reinterprets the bit pattern.

_Defined in `lang/std/num/int16.ks`._

#### initializer `From Integer`

```kestrel
public init(from: Int32)
```

Converts from `Int32`. Narrowing conversions truncate the high
bits; signed→unsigned reinterprets the bit pattern.

_Defined in `lang/std/num/int16.ks`._

#### initializer `From Integer`

```kestrel
public init(from: Int64)
```

Converts from `Int64`. Narrowing conversions truncate the high
bits; signed→unsigned reinterprets the bit pattern.

_Defined in `lang/std/num/int16.ks`._

#### initializer `From Integer`

```kestrel
public init(from: UInt8)
```

Converts from `UInt8`. Narrowing conversions truncate the high
bits; signed→unsigned reinterprets the bit pattern.

_Defined in `lang/std/num/int16.ks`._

#### initializer `From Integer`

```kestrel
public init(from: UInt16)
```

Converts from `UInt16`. Narrowing conversions truncate the high
bits; signed→unsigned reinterprets the bit pattern.

_Defined in `lang/std/num/int16.ks`._

#### initializer `From Integer`

```kestrel
public init(from: UInt32)
```

Converts from `UInt32`. Narrowing conversions truncate the high
bits; signed→unsigned reinterprets the bit pattern.

_Defined in `lang/std/num/int16.ks`._

#### initializer `From Integer`

```kestrel
public init(from: UInt64)
```

Converts from `UInt64`. Narrowing conversions truncate the high
bits; signed→unsigned reinterprets the bit pattern.

_Defined in `lang/std/num/int16.ks`._

#### initializer `From Raw`

```kestrel
init(raw: lang.i16)
```

Wraps an existing `lang.i16` without conversion. Internal
constructor used by intrinsics; not part of the public API.

_Defined in `lang/std/num/int16.ks`._

#### initializer `Int Literal`

```kestrel
public init(intLiteral: lang.i64)
```

Compiler-emitted bridge that turns an integer literal into a Int16.

You will rarely call this directly — write the literal and let the
`ExpressibleByIntLiteral` protocol pick it up. For widths smaller than
64 bits the literal is truncated with `lang.cast_i64_i16`.

##### Examples

```
let n: Int64 = 42;            // implicit
let m = Int64(intLiteral: 42);  // explicit
```

_Defined in `lang/std/num/int16.ks`._

#### function `absChecked`

```kestrel
public func absChecked() -> Int16?
```

Absolute value that returns `None` for `minValue` (whose absolute
value overflows).

_Defined in `lang/std/num/int16.ks`._

#### function `absSaturating`

```kestrel
public func absSaturating() -> Int16
```

Absolute value that returns `maxValue` instead of wrapping `minValue`.

_Defined in `lang/std/num/int16.ks`._

#### function `addChecked`

```kestrel
public func addChecked(Int16) -> Int16?
```

Wrapping addition that returns `None` instead of overflowing.

_Defined in `lang/std/num/int16.ks`._

#### function `addSaturating`

```kestrel
public func addSaturating(Int16) -> Int16
```

Addition that clamps to `maxValue`/`minValue` instead of wrapping.

_Defined in `lang/std/num/int16.ks`._

#### field `bitWidth`

```kestrel
public static var bitWidth: Int64 { get }
```

The width in bits (16). Useful for shift bounds and bit-walks.

_Defined in `lang/std/num/int16.ks`._

#### field `byteSwapped`

```kestrel
public var byteSwapped: Int16 { get }
```

Value with its byte order reversed. Use to convert between big- and
little-endian; lowered to a `bswap` intrinsic.

_Defined in `lang/std/num/int16.ks`._

#### function `clamp`

```kestrel
public func clamp(Int16, Int16) -> Int16
```

Clamps `self` into `[min, max]`. Caller is responsible for ensuring
`min <= max`; otherwise the result is undefined.

##### Examples

```
(5).clamp(min: 0, max: 10);    // 5
(-5).clamp(min: 0, max: 10);   // 0
(15).clamp(min: 0, max: 10);   // 10
```

_Defined in `lang/std/num/int16.ks`._

#### field `countOnes`

```kestrel
public var countOnes: Int64 { get }
```

Population count — the number of `1` bits in the binary representation.

Lowered to a `popcount` intrinsic where the target supports it.

##### Examples

```
(0b1010).countOnes;  // 2
(0b1111).countOnes;  // 4
(0).countOnes;       // 0
```

_Defined in `lang/std/num/int16.ks`._

#### field `countZeros`

```kestrel
public var countZeros: Int64 { get }
```

Complement of `countOnes`: equal to `bitWidth - countOnes`.

_Defined in `lang/std/num/int16.ks`._

#### function `divideChecked`

```kestrel
public func divideChecked(Int16) -> Int16?
```

Division that returns `None` for divide-by-zero or for the
`minValue / -1` overflow case.

_Defined in `lang/std/num/int16.ks`._

#### function `fromBytes`

```kestrel
public static func fromBytes(std.collections.Array[UInt8]) -> Int16?
```

Reassembles a `Int16` from 2 bytes in native (host) byte
order. Returns `None` if the input is not exactly 2 bytes long.

_Defined in `lang/std/num/int16.ks`._

#### function `fromBytesBigEndian`

```kestrel
public static func fromBytesBigEndian(std.collections.Array[UInt8]) -> Int16?
```

Reassembles a `Int16` from 2 bytes in big-endian order.
Returns `None` if the input is not exactly 2 bytes long.

_Defined in `lang/std/num/int16.ks`._

#### function `fromBytesLittleEndian`

```kestrel
public static func fromBytesLittleEndian(std.collections.Array[UInt8]) -> Int16?
```

Reassembles a `Int16` from 2 bytes in little-endian order.
Returns `None` if the input is not exactly 2 bytes long.

_Defined in `lang/std/num/int16.ks`._

#### function `gcd`

```kestrel
public func gcd(Int16) -> Int16
```

Greatest common divisor via Euclidean algorithm. For signed types
the inputs are taken absolute first; the result is always non-negative.

##### Examples

```
(12).gcd(8);   // 4
(17).gcd(5);   // 1   (coprime)
(-12).gcd(8);  // 4
```

_Defined in `lang/std/num/int16.ks`._

#### field `isNegative`

```kestrel
public var isNegative: Bool { get }
```

True when `self < 0`.

_Defined in `lang/std/num/int16.ks`._

#### field `isPositive`

```kestrel
public var isPositive: Bool { get }
```

True when `self > 0`.

_Defined in `lang/std/num/int16.ks`._

#### field `isPowerOfTwo`

```kestrel
public var isPowerOfTwo: Bool { get }
```

True when the value is a positive power of two (`2^k` for `k >= 0`).

Zero and negatives are excluded. Cheap branchless test built on
`x & (x - 1) == 0`.

##### Examples

```
(1).isPowerOfTwo;   // true  (2^0)
(4).isPowerOfTwo;   // true  (2^2)
(3).isPowerOfTwo;   // false
(0).isPowerOfTwo;   // false
```

_Defined in `lang/std/num/int16.ks`._

#### field `isZero`

```kestrel
public var isZero: Bool { get }
```

True when `self == 0`.

_Defined in `lang/std/num/int16.ks`._

#### function `lcm`

```kestrel
public func lcm(Int16) -> Int16
```

Least common multiple, computed as `|self| / gcd(self, other) * |other|`
to avoid intermediate overflow. Returns zero if either input is zero.

##### Examples

```
(4).lcm(6);   // 12
(3).lcm(5);   // 15
(0).lcm(7);   // 0
```

_Defined in `lang/std/num/int16.ks`._

#### field `leadingZeros`

```kestrel
public var leadingZeros: Int64 { get }
```

Number of leading zero bits, counting from the most-significant end.

For zero, returns `bitWidth`.

##### Examples

```
(1).leadingZeros;   // bitWidth - 1
(0).leadingZeros;   // bitWidth
```

_Defined in `lang/std/num/int16.ks`._

#### field `maxValue`

```kestrel
public static var maxValue: Int16 { get }
```

The largest representable value.
This is 2^15 - 1 (32_767).

_Defined in `lang/std/num/int16.ks`._

#### field `minValue`

```kestrel
public static var minValue: Int16 { get }
```

The smallest representable value.
This is -2^15 (-32_768).
Note that for signed types `minValue.negate()` overflows back to
itself; use `negateChecked()` if you need to detect that.

_Defined in `lang/std/num/int16.ks`._

#### function `multiplyChecked`

```kestrel
public func multiplyChecked(Int16) -> Int16?
```

Wrapping multiplication that returns `None` instead of overflowing.
Implemented by multiplying then dividing back; replace with an
overflow-detecting intrinsic when one is available.

_Defined in `lang/std/num/int16.ks`._

#### function `multiplySaturating`

```kestrel
public func multiplySaturating(Int16) -> Int16
```

Multiplication that clamps to `maxValue`/`minValue` instead of wrapping.
The clamp direction follows the algebraic sign of the would-be result.

_Defined in `lang/std/num/int16.ks`._

#### function `negateChecked`

```kestrel
public func negateChecked() -> Int16?
```

Negation that returns `None` for `minValue` (whose negation overflows).

_Defined in `lang/std/num/int16.ks`._

#### function `negateSaturating`

```kestrel
public func negateSaturating() -> Int16
```

Negation that returns `maxValue` instead of wrapping `minValue`.

_Defined in `lang/std/num/int16.ks`._

#### function `parse`

```kestrel
public static func parse(String) -> Int16?
```

Parses a base-10 integer literal, optionally prefixed with `+` or
`-`. Returns `None` for an empty string, a non-digit character,
or a value that does not fit in `Int16`.

##### Examples

```
Int16.parse(string: "42");    // Some(42)
Int16.parse(string: "-7");    // Some(-7)
Int16.parse(string: "abc");   // None
Int16.parse(string: "");      // None
```

_Defined in `lang/std/num/int16.ks`._

#### function `parse`

```kestrel
public static func parse(String, Int64) -> Int16?
```

Parses an integer in `radix` (base 2–36 inclusive). Letters a–z are
case-insensitive and represent digit values 10–35. Returns `None`
for an out-of-range radix, an empty string, an unrecognised digit,
or a value that overflows `Int16`.

##### Examples

```
Int16.parse(string: "ff", radix: 16);     // Some(255 if it fits, else None)
Int16.parse(string: "101010", radix: 2);  // Some(42)
Int16.parse(string: "z", radix: 36);      // Some(35)
```

_Defined in `lang/std/num/int16.ks`._

#### function `pow`

```kestrel
public func pow(Int64) -> Int16
```

Raises `self` to `exponent` via binary exponentiation. Wraps on
overflow. Negative exponents return zero (integer truncation of
the would-be fraction).

##### Examples

```
(2).pow(10);  // 1024
(3).pow(4);   // 81
(5).pow(-1);  // 0
```

_Defined in `lang/std/num/int16.ks`._

#### field `raw`

```kestrel
public var raw: lang.i16
```

The underlying primitive `lang.i16` value. Exposed for FFI
and intrinsic use; prefer the typed surface for everything else.

_Defined in `lang/std/num/int16.ks`._

#### function `rotateLeft`

```kestrel
public func rotateLeft(by: Int64) -> Int16
```

Rotates bits left by `count`, modulo `bitWidth`. Bits shifted past the
MSB re-enter at the LSB.

_Defined in `lang/std/num/int16.ks`._

#### function `rotateRight`

```kestrel
public func rotateRight(by: Int64) -> Int16
```

Rotates bits right by `count`, modulo `bitWidth`. Mirror of
`rotateLeft`.

_Defined in `lang/std/num/int16.ks`._

#### field `sign`

```kestrel
public var sign: Int16 { get }
```

Sign as a `Int16`: `-1`, `0`, or `1`.

_Defined in `lang/std/num/int16.ks`._

#### function `subtractChecked`

```kestrel
public func subtractChecked(Int16) -> Int16?
```

Wrapping subtraction that returns `None` instead of overflowing.

_Defined in `lang/std/num/int16.ks`._

#### function `subtractSaturating`

```kestrel
public func subtractSaturating(Int16) -> Int16
```

Subtraction that clamps to `maxValue`/`minValue` instead of wrapping.

_Defined in `lang/std/num/int16.ks`._

#### function `toBytes`

```kestrel
public func toBytes() -> std.collections.Array[UInt8]
```

Splits this integer into 2 bytes in *native* (host) byte order.
Use `toBytesBigEndian` / `toBytesLittleEndian` when serialising for
a fixed wire format.

##### Examples

```
let bytes = Int16.maxValue.toBytes();   // 2 bytes, host order
```

_Defined in `lang/std/num/int16.ks`._

#### function `toBytesBigEndian`

```kestrel
public func toBytesBigEndian() -> std.collections.Array[UInt8]
```

Splits this integer into 2 bytes in big-endian order (most
significant byte first — i.e. network byte order).

_Defined in `lang/std/num/int16.ks`._

#### function `toBytesLittleEndian`

```kestrel
public func toBytesLittleEndian() -> std.collections.Array[UInt8]
```

Splits this integer into 2 bytes in little-endian order (least
significant byte first).

_Defined in `lang/std/num/int16.ks`._

#### field `trailingZeros`

```kestrel
public var trailingZeros: Int64 { get }
```

Number of trailing zero bits. Equal to `log2(self & -self)` for non-zero
values; returns `bitWidth` for zero. Useful for finding the largest
power of two dividing the value.

_Defined in `lang/std/num/int16.ks`._

### Implements `SignedInteger`

#### function `abs`

```kestrel
public func abs() -> Int16
```

Absolute value. Wraps at the minimum value
(`Int16.minValue.abs() == Int16.minValue`); use
`absChecked` if that's a problem.

_Defined in `lang/std/num/int16.ks`._

### Implements `Steppable`

#### function `predecessor`

```kestrel
public func predecessor() -> Int16
```

Predecessor — `self - 1`. Wraps at `minValue`.

_Defined in `lang/std/num/int16.ks`._

#### function `successor`

```kestrel
public func successor() -> Int16
```

Successor — `self + 1`. Wraps at `maxValue`. Used by `for-in` over
integer ranges.

_Defined in `lang/std/num/int16.ks`._

### Implements `Comparable`

#### function `compare`

```kestrel
public func compare(Int16) -> Ordering
```

Three-way comparison returning an `Ordering`. Signed types compare
using two's-complement ordering; unsigned types use natural ordering.

##### Examples

```
(1).compare(other: 2);   // .Less
(2).compare(other: 2);   // .Equal
(3).compare(other: 2);   // .Greater
```

_Defined in `lang/std/num/int16.ks`._

### Implements `Equatable`

#### function `equals`

```kestrel
public func equals(Int16) -> Bool
```

Bit-for-bit equality. Backs the `==` operator.

##### Examples

```
(42).equals(other: 42);  // true
42 == 42;                // true
```

_Defined in `lang/std/num/int16.ks`._

### Implements `Matchable`

#### function `matches`

```kestrel
public func matches(Int16) -> Bool
```

Pattern-matching hook for `Matchable`. Identical to `equals`.

_Defined in `lang/std/num/int16.ks`._

### Implements `Formattable`

#### function `format`

```kestrel
public func format(FormatOptions) -> String
```

Renders the integer to a `String`, honouring the supplied
`FormatOptions`. Implements the `Formattable` protocol.

Recognised options:
- `radix` — base in `[2, 36]`; out-of-range values fall back to 10.
- `width` — minimum output width; shorter values are padded.
- `fill` / `alignment` — padding character and side.
- `sign` — `.Negative` (default), `.Always`, or `.Space`.
- `uppercase` — uppercase hex digits.
- `alternate` — emit the `0b` / `0o` / `0x` prefix.

##### Examples

```
(42).format();                                           // "42"
(255).format(options: .{radix: 16});                     // "ff"
(255).format(options: .{radix: 16, uppercase: true});    // "FF"
(255).format(options: .{radix: 16, alternate: true});    // "0xff"
(42).format(options: .{radix: 2, alternate: true});      // "0b101010"
(42).format(options: .{width: .Some(5), fill: '0'});     // "00042"
(-42).format(options: .{sign: .Always});                 // "-42"
```

_Defined in `lang/std/num/int16.ks`._

### Implements `Hash`

#### function `hash`

```kestrel
public func hash[H](into: mutating H) where H: Hasher
```

Feeds the raw bytes of this value into `hasher`. Endianness-agnostic
only within a single process — do not persist hashes across builds.

_Defined in `lang/std/num/int16.ks`._

### Implements `Addable`

#### typealias `Output`

```kestrel
type Output = Int16
```

_Defined in `lang/std/num/int16.ks`._

#### typealias `Output`

```kestrel
type Output = Int16
```

_Defined in `lang/std/num/int16.ks`._

#### typealias `Output`

```kestrel
type Output = Int16
```

_Defined in `lang/std/num/int16.ks`._

#### typealias `Output`

```kestrel
type Output = Int16
```

_Defined in `lang/std/num/int16.ks`._

#### typealias `Output`

```kestrel
type Output = Int16
```

_Defined in `lang/std/num/int16.ks`._

#### typealias `Output`

```kestrel
type Output = Int16
```

_Defined in `lang/std/num/int16.ks`._

#### typealias `Output`

```kestrel
type Output = Int16
```

_Defined in `lang/std/num/int16.ks`._

#### typealias `Output`

```kestrel
type Output = Int16
```

_Defined in `lang/std/num/int16.ks`._

#### typealias `Output`

```kestrel
type Output = Int16
```

_Defined in `lang/std/num/int16.ks`._

#### typealias `Output`

```kestrel
type Output = Int16
```

_Defined in `lang/std/num/int16.ks`._

#### typealias `Output`

```kestrel
type Output = Int16
```

_Defined in `lang/std/num/int16.ks`._

#### typealias `Output`

```kestrel
type Output = Int16
```

_Defined in `lang/std/num/int16.ks`._

#### typealias `Output`

```kestrel
type Output = Range[Int16]
```

_Defined in `lang/std/num/int16.ks`._

#### typealias `Output`

```kestrel
type Output = ClosedRange[Int16]
```

_Defined in `lang/std/num/int16.ks`._

#### function `add`

```kestrel
public func add(Int16) -> Int16
```

`self + other`, wrapping on overflow. Use `addChecked` to detect or
`addSaturating` to clamp.

_Defined in `lang/std/num/int16.ks`._

#### field `zero`

```kestrel
public static var zero: Int16 { get }
```

The additive identity, `0`.

_Defined in `lang/std/num/int16.ks`._

### Implements `Subtractable`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/arithmetic.ks`._

#### function `subtract`

```kestrel
public func subtract(Int16) -> Int16
```

`self - other`, wrapping on overflow.

_Defined in `lang/std/num/int16.ks`._

### Implements `Multipliable`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/arithmetic.ks`._

#### function `multiply`

```kestrel
public func multiply(Int16) -> Int16
```

`self * other`, wrapping on overflow.

_Defined in `lang/std/num/int16.ks`._

#### field `one`

```kestrel
public static var one: Int16 { get }
```

The multiplicative identity, `1`.

_Defined in `lang/std/num/int16.ks`._

### Implements `Divisible`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/arithmetic.ks`._

#### function `divide`

```kestrel
public func divide(Int16) -> Int16
```

Truncating integer division (`self / other`). For signed types,
`minValue / -1` wraps; use `divideChecked` to detect.

##### Errors

Traps on division by zero (LLVM `udiv`/`sdiv` are UB on zero — the
process aborts before producing a result).

_Defined in `lang/std/num/int16.ks`._

### Implements `Modulo`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/arithmetic.ks`._

#### function `modulo`

```kestrel
public func modulo(Int16) -> Int16
```

`self % other` — truncated remainder; the result has the sign of
`self` for signed types.

##### Errors

Traps on division by zero, like `divide`.

_Defined in `lang/std/num/int16.ks`._

### Implements `Negatable`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/arithmetic.ks`._

#### function `negate`

```kestrel
public func negate() -> Int16
```

Two's-complement negation. Wraps at the minimum value:
`Int16.minValue.negate() == Int16.minValue`. Use
`negateChecked` to surface the overflow.

_Defined in `lang/std/num/int16.ks`._

### Implements `BitwiseAnd`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/bitwise.ks`._

#### function `bitwiseAnd`

```kestrel
public func bitwiseAnd(Int16) -> Int16
```

Bitwise AND. `0b1010 & 0b1100 == 0b1000`.

_Defined in `lang/std/num/int16.ks`._

### Implements `BitwiseOr`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/bitwise.ks`._

#### function `bitwiseOr`

```kestrel
public func bitwiseOr(Int16) -> Int16
```

Bitwise OR. `0b1010 | 0b1100 == 0b1110`.

_Defined in `lang/std/num/int16.ks`._

### Implements `BitwiseXor`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/bitwise.ks`._

#### function `bitwiseXor`

```kestrel
public func bitwiseXor(Int16) -> Int16
```

Bitwise XOR. `0b1010 ^ 0b1100 == 0b0110`.

_Defined in `lang/std/num/int16.ks`._

### Implements `BitwiseNot`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/bitwise.ks`._

#### function `bitwiseNot`

```kestrel
public func bitwiseNot() -> Int16
```

Bitwise NOT — flips all bits. For signed types this is `-self - 1`.

_Defined in `lang/std/num/int16.ks`._

### Implements `LeftShift`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/bitwise.ks`._

#### function `shiftLeft`

```kestrel
public func shiftLeft(by: lang.i64) -> Int16
```

Left shift by `count`. Behavior is undefined when `count >= bitWidth`
— pre-mask the count if you can't guarantee the bound.

_Defined in `lang/std/num/int16.ks`._

### Implements `RightShift`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/bitwise.ks`._

#### function `shiftRight`

```kestrel
public func shiftRight(by: lang.i64) -> Int16
```

Right shift by `count`. Arithmetic (sign-extending) for signed types,
logical (zero-filling) for unsigned. Same `count` precondition as
`shiftLeft`.

_Defined in `lang/std/num/int16.ks`._

### Implements `AddAssign`

#### function `addAssign`

```kestrel
public mutating func addAssign(Int16)
```

`self += other`

_Defined in `lang/std/num/int16.ks`._

### Implements `SubtractAssign`

#### function `subtractAssign`

```kestrel
public mutating func subtractAssign(Int16)
```

`self -= other`

_Defined in `lang/std/num/int16.ks`._

### Implements `MultiplyAssign`

#### function `multiplyAssign`

```kestrel
public mutating func multiplyAssign(Int16)
```

`self *= other`

_Defined in `lang/std/num/int16.ks`._

### Implements `DivideAssign`

#### function `divideAssign`

```kestrel
public mutating func divideAssign(Int16)
```

`self /= other`

_Defined in `lang/std/num/int16.ks`._

### Implements `ModuloAssign`

#### function `modAssign`

```kestrel
public mutating func modAssign(Int16)
```

`self %= other`

_Defined in `lang/std/num/int16.ks`._

### Implements `BitwiseAndAssign`

#### function `bitwiseAndAssign`

```kestrel
public mutating func bitwiseAndAssign(Int16)
```

`self &= other`

_Defined in `lang/std/num/int16.ks`._

### Implements `BitwiseOrAssign`

#### function `bitwiseOrAssign`

```kestrel
public mutating func bitwiseOrAssign(Int16)
```

`self |= other`

_Defined in `lang/std/num/int16.ks`._

### Implements `BitwiseXorAssign`

#### function `bitwiseXorAssign`

```kestrel
public mutating func bitwiseXorAssign(Int16)
```

`self ^= other`

_Defined in `lang/std/num/int16.ks`._

### Implements `LeftShiftAssign`

#### function `shiftLeftAssign`

```kestrel
public mutating func shiftLeftAssign(by: lang.i64)
```

`self <<= count`

_Defined in `lang/std/num/int16.ks`._

### Implements `RightShiftAssign`

#### function `shiftRightAssign`

```kestrel
public mutating func shiftRightAssign(by: lang.i64)
```

`self >>= count`

_Defined in `lang/std/num/int16.ks`._

### Implements `ExpressibleByIntLiteral`

#### initializer `Int Literal`

```kestrel
init(intLiteral: lang.i64)
```

Builds an instance from an integer literal.

_Defined in `lang/std/core/literals.ks`._

### Implements `Defaultable`

#### initializer `Default`

```kestrel
init()
```

Builds the default-valued instance.

_Defined in `lang/std/core/protocols.ks`._

### Implements `RangeConstructible`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/range.ks`._

#### function `exclusiveRange`

```kestrel
public func exclusiveRange(to: Int16) -> Range[Int16]
```

Builds a half-open range `self..<end`. Sugar for the `..<` operator.

_Defined in `lang/std/num/int16.ks`._

### Implements `ClosedRangeConstructible`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/range.ks`._

#### function `inclusiveRange`

```kestrel
public func inclusiveRange(to: Int16) -> ClosedRange[Int16]
```

Builds a closed range `self..=end`. Sugar for the `..=` operator.

_Defined in `lang/std/num/int16.ks`._

### Implements `Convertible`

#### initializer `From Source`

```kestrel
init(from: From)
```

Creates an instance from `value`.

_Defined in `lang/std/core/convertible.ks`._

## struct `Int32`

```kestrel
public struct Int32 { /* private fields */ }
```

A 32-bit signed integer.

Int32 is the 32-bit member of the integer family. The same surface
area is provided across all widths; switch widths to trade range for memory
or to match an FFI ABI. Arithmetic wraps on overflow by default — use the
`*Checked` variants for overflow detection or `*Saturating` to clamp to
`minValue`/`maxValue`. The type is `FFISafe` and lays out as a single
`lang.i32` so it can cross C boundaries unchanged.

### Examples

```
let a: Int64 = 100;
let b = a + 50;        // 150
let c = a * 2;         // 200
let d = a.addChecked(Int64.maxValue);  // None (overflow detected)
```

```
// Bit twiddling
(0b1010).countOnes      // 2
(1).shiftLeft(by: 4)    // 16
(-1).leadingZeros       // 0  (all bits set)
```

### Representation

A single `lang.i32` field. No padding, no headers — bit-identical
to the corresponding C type.

_Defined in `lang/std/num/int32.ks`._

### Members

#### initializer `Default`

```kestrel
public init()
```

Creates the zero value, satisfying `Defaultable`.

##### Examples

```
let n = Int64();   // 0
```

_Defined in `lang/std/num/int32.ks`._

#### initializer `From Integer`

```kestrel
public init(from: Int8)
```

Converts from `Int8`. Narrowing conversions truncate the high
bits; signed→unsigned reinterprets the bit pattern.

_Defined in `lang/std/num/int32.ks`._

#### initializer `From Integer`

```kestrel
public init(from: Int16)
```

Converts from `Int16`. Narrowing conversions truncate the high
bits; signed→unsigned reinterprets the bit pattern.

_Defined in `lang/std/num/int32.ks`._

#### initializer `From Integer`

```kestrel
public init(from: Int64)
```

Converts from `Int64`. Narrowing conversions truncate the high
bits; signed→unsigned reinterprets the bit pattern.

_Defined in `lang/std/num/int32.ks`._

#### initializer `From Integer`

```kestrel
public init(from: UInt8)
```

Converts from `UInt8`. Narrowing conversions truncate the high
bits; signed→unsigned reinterprets the bit pattern.

_Defined in `lang/std/num/int32.ks`._

#### initializer `From Integer`

```kestrel
public init(from: UInt16)
```

Converts from `UInt16`. Narrowing conversions truncate the high
bits; signed→unsigned reinterprets the bit pattern.

_Defined in `lang/std/num/int32.ks`._

#### initializer `From Integer`

```kestrel
public init(from: UInt32)
```

Converts from `UInt32`. Narrowing conversions truncate the high
bits; signed→unsigned reinterprets the bit pattern.

_Defined in `lang/std/num/int32.ks`._

#### initializer `From Integer`

```kestrel
public init(from: UInt64)
```

Converts from `UInt64`. Narrowing conversions truncate the high
bits; signed→unsigned reinterprets the bit pattern.

_Defined in `lang/std/num/int32.ks`._

#### initializer `From Raw`

```kestrel
init(raw: lang.i32)
```

Wraps an existing `lang.i32` without conversion. Internal
constructor used by intrinsics; not part of the public API.

_Defined in `lang/std/num/int32.ks`._

#### initializer `Int Literal`

```kestrel
public init(intLiteral: lang.i64)
```

Compiler-emitted bridge that turns an integer literal into a Int32.

You will rarely call this directly — write the literal and let the
`ExpressibleByIntLiteral` protocol pick it up. For widths smaller than
64 bits the literal is truncated with `lang.cast_i64_i32`.

##### Examples

```
let n: Int64 = 42;            // implicit
let m = Int64(intLiteral: 42);  // explicit
```

_Defined in `lang/std/num/int32.ks`._

#### function `absChecked`

```kestrel
public func absChecked() -> Int32?
```

Absolute value that returns `None` for `minValue` (whose absolute
value overflows).

_Defined in `lang/std/num/int32.ks`._

#### function `absSaturating`

```kestrel
public func absSaturating() -> Int32
```

Absolute value that returns `maxValue` instead of wrapping `minValue`.

_Defined in `lang/std/num/int32.ks`._

#### function `addChecked`

```kestrel
public func addChecked(Int32) -> Int32?
```

Wrapping addition that returns `None` instead of overflowing.

_Defined in `lang/std/num/int32.ks`._

#### function `addSaturating`

```kestrel
public func addSaturating(Int32) -> Int32
```

Addition that clamps to `maxValue`/`minValue` instead of wrapping.

_Defined in `lang/std/num/int32.ks`._

#### field `bitWidth`

```kestrel
public static var bitWidth: Int64 { get }
```

The width in bits (32). Useful for shift bounds and bit-walks.

_Defined in `lang/std/num/int32.ks`._

#### field `byteSwapped`

```kestrel
public var byteSwapped: Int32 { get }
```

Value with its byte order reversed. Use to convert between big- and
little-endian; lowered to a `bswap` intrinsic.

_Defined in `lang/std/num/int32.ks`._

#### function `clamp`

```kestrel
public func clamp(Int32, Int32) -> Int32
```

Clamps `self` into `[min, max]`. Caller is responsible for ensuring
`min <= max`; otherwise the result is undefined.

##### Examples

```
(5).clamp(min: 0, max: 10);    // 5
(-5).clamp(min: 0, max: 10);   // 0
(15).clamp(min: 0, max: 10);   // 10
```

_Defined in `lang/std/num/int32.ks`._

#### field `countOnes`

```kestrel
public var countOnes: Int64 { get }
```

Population count — the number of `1` bits in the binary representation.

Lowered to a `popcount` intrinsic where the target supports it.

##### Examples

```
(0b1010).countOnes;  // 2
(0b1111).countOnes;  // 4
(0).countOnes;       // 0
```

_Defined in `lang/std/num/int32.ks`._

#### field `countZeros`

```kestrel
public var countZeros: Int64 { get }
```

Complement of `countOnes`: equal to `bitWidth - countOnes`.

_Defined in `lang/std/num/int32.ks`._

#### function `divideChecked`

```kestrel
public func divideChecked(Int32) -> Int32?
```

Division that returns `None` for divide-by-zero or for the
`minValue / -1` overflow case.

_Defined in `lang/std/num/int32.ks`._

#### function `fromBytes`

```kestrel
public static func fromBytes(std.collections.Array[UInt8]) -> Int32?
```

Reassembles a `Int32` from 4 bytes in native (host) byte
order. Returns `None` if the input is not exactly 4 bytes long.

_Defined in `lang/std/num/int32.ks`._

#### function `fromBytesBigEndian`

```kestrel
public static func fromBytesBigEndian(std.collections.Array[UInt8]) -> Int32?
```

Reassembles a `Int32` from 4 bytes in big-endian order.
Returns `None` if the input is not exactly 4 bytes long.

_Defined in `lang/std/num/int32.ks`._

#### function `fromBytesLittleEndian`

```kestrel
public static func fromBytesLittleEndian(std.collections.Array[UInt8]) -> Int32?
```

Reassembles a `Int32` from 4 bytes in little-endian order.
Returns `None` if the input is not exactly 4 bytes long.

_Defined in `lang/std/num/int32.ks`._

#### function `gcd`

```kestrel
public func gcd(Int32) -> Int32
```

Greatest common divisor via Euclidean algorithm. For signed types
the inputs are taken absolute first; the result is always non-negative.

##### Examples

```
(12).gcd(8);   // 4
(17).gcd(5);   // 1   (coprime)
(-12).gcd(8);  // 4
```

_Defined in `lang/std/num/int32.ks`._

#### field `isNegative`

```kestrel
public var isNegative: Bool { get }
```

True when `self < 0`.

_Defined in `lang/std/num/int32.ks`._

#### field `isPositive`

```kestrel
public var isPositive: Bool { get }
```

True when `self > 0`.

_Defined in `lang/std/num/int32.ks`._

#### field `isPowerOfTwo`

```kestrel
public var isPowerOfTwo: Bool { get }
```

True when the value is a positive power of two (`2^k` for `k >= 0`).

Zero and negatives are excluded. Cheap branchless test built on
`x & (x - 1) == 0`.

##### Examples

```
(1).isPowerOfTwo;   // true  (2^0)
(4).isPowerOfTwo;   // true  (2^2)
(3).isPowerOfTwo;   // false
(0).isPowerOfTwo;   // false
```

_Defined in `lang/std/num/int32.ks`._

#### field `isZero`

```kestrel
public var isZero: Bool { get }
```

True when `self == 0`.

_Defined in `lang/std/num/int32.ks`._

#### function `lcm`

```kestrel
public func lcm(Int32) -> Int32
```

Least common multiple, computed as `|self| / gcd(self, other) * |other|`
to avoid intermediate overflow. Returns zero if either input is zero.

##### Examples

```
(4).lcm(6);   // 12
(3).lcm(5);   // 15
(0).lcm(7);   // 0
```

_Defined in `lang/std/num/int32.ks`._

#### field `leadingZeros`

```kestrel
public var leadingZeros: Int64 { get }
```

Number of leading zero bits, counting from the most-significant end.

For zero, returns `bitWidth`.

##### Examples

```
(1).leadingZeros;   // bitWidth - 1
(0).leadingZeros;   // bitWidth
```

_Defined in `lang/std/num/int32.ks`._

#### field `maxValue`

```kestrel
public static var maxValue: Int32 { get }
```

The largest representable value.
This is 2^31 - 1 (2_147_483_647).

_Defined in `lang/std/num/int32.ks`._

#### field `minValue`

```kestrel
public static var minValue: Int32 { get }
```

The smallest representable value.
This is -2^31 (-2_147_483_648).
Note that for signed types `minValue.negate()` overflows back to
itself; use `negateChecked()` if you need to detect that.

_Defined in `lang/std/num/int32.ks`._

#### function `multiplyChecked`

```kestrel
public func multiplyChecked(Int32) -> Int32?
```

Wrapping multiplication that returns `None` instead of overflowing.
Implemented by multiplying then dividing back; replace with an
overflow-detecting intrinsic when one is available.

_Defined in `lang/std/num/int32.ks`._

#### function `multiplySaturating`

```kestrel
public func multiplySaturating(Int32) -> Int32
```

Multiplication that clamps to `maxValue`/`minValue` instead of wrapping.
The clamp direction follows the algebraic sign of the would-be result.

_Defined in `lang/std/num/int32.ks`._

#### function `negateChecked`

```kestrel
public func negateChecked() -> Int32?
```

Negation that returns `None` for `minValue` (whose negation overflows).

_Defined in `lang/std/num/int32.ks`._

#### function `negateSaturating`

```kestrel
public func negateSaturating() -> Int32
```

Negation that returns `maxValue` instead of wrapping `minValue`.

_Defined in `lang/std/num/int32.ks`._

#### function `parse`

```kestrel
public static func parse(String) -> Int32?
```

Parses a base-10 integer literal, optionally prefixed with `+` or
`-`. Returns `None` for an empty string, a non-digit character,
or a value that does not fit in `Int32`.

##### Examples

```
Int32.parse(string: "42");    // Some(42)
Int32.parse(string: "-7");    // Some(-7)
Int32.parse(string: "abc");   // None
Int32.parse(string: "");      // None
```

_Defined in `lang/std/num/int32.ks`._

#### function `parse`

```kestrel
public static func parse(String, Int64) -> Int32?
```

Parses an integer in `radix` (base 2–36 inclusive). Letters a–z are
case-insensitive and represent digit values 10–35. Returns `None`
for an out-of-range radix, an empty string, an unrecognised digit,
or a value that overflows `Int32`.

##### Examples

```
Int32.parse(string: "ff", radix: 16);     // Some(255 if it fits, else None)
Int32.parse(string: "101010", radix: 2);  // Some(42)
Int32.parse(string: "z", radix: 36);      // Some(35)
```

_Defined in `lang/std/num/int32.ks`._

#### function `pow`

```kestrel
public func pow(Int64) -> Int32
```

Raises `self` to `exponent` via binary exponentiation. Wraps on
overflow. Negative exponents return zero (integer truncation of
the would-be fraction).

##### Examples

```
(2).pow(10);  // 1024
(3).pow(4);   // 81
(5).pow(-1);  // 0
```

_Defined in `lang/std/num/int32.ks`._

#### field `raw`

```kestrel
public var raw: lang.i32
```

The underlying primitive `lang.i32` value. Exposed for FFI
and intrinsic use; prefer the typed surface for everything else.

_Defined in `lang/std/num/int32.ks`._

#### function `rotateLeft`

```kestrel
public func rotateLeft(by: Int64) -> Int32
```

Rotates bits left by `count`, modulo `bitWidth`. Bits shifted past the
MSB re-enter at the LSB.

_Defined in `lang/std/num/int32.ks`._

#### function `rotateRight`

```kestrel
public func rotateRight(by: Int64) -> Int32
```

Rotates bits right by `count`, modulo `bitWidth`. Mirror of
`rotateLeft`.

_Defined in `lang/std/num/int32.ks`._

#### field `sign`

```kestrel
public var sign: Int32 { get }
```

Sign as a `Int32`: `-1`, `0`, or `1`.

_Defined in `lang/std/num/int32.ks`._

#### function `subtractChecked`

```kestrel
public func subtractChecked(Int32) -> Int32?
```

Wrapping subtraction that returns `None` instead of overflowing.

_Defined in `lang/std/num/int32.ks`._

#### function `subtractSaturating`

```kestrel
public func subtractSaturating(Int32) -> Int32
```

Subtraction that clamps to `maxValue`/`minValue` instead of wrapping.

_Defined in `lang/std/num/int32.ks`._

#### function `toBytes`

```kestrel
public func toBytes() -> std.collections.Array[UInt8]
```

Splits this integer into 4 bytes in *native* (host) byte order.
Use `toBytesBigEndian` / `toBytesLittleEndian` when serialising for
a fixed wire format.

##### Examples

```
let bytes = Int32.maxValue.toBytes();   // 4 bytes, host order
```

_Defined in `lang/std/num/int32.ks`._

#### function `toBytesBigEndian`

```kestrel
public func toBytesBigEndian() -> std.collections.Array[UInt8]
```

Splits this integer into 4 bytes in big-endian order (most
significant byte first — i.e. network byte order).

_Defined in `lang/std/num/int32.ks`._

#### function `toBytesLittleEndian`

```kestrel
public func toBytesLittleEndian() -> std.collections.Array[UInt8]
```

Splits this integer into 4 bytes in little-endian order (least
significant byte first).

_Defined in `lang/std/num/int32.ks`._

#### field `trailingZeros`

```kestrel
public var trailingZeros: Int64 { get }
```

Number of trailing zero bits. Equal to `log2(self & -self)` for non-zero
values; returns `bitWidth` for zero. Useful for finding the largest
power of two dividing the value.

_Defined in `lang/std/num/int32.ks`._

### Implements `SignedInteger`

#### function `abs`

```kestrel
public func abs() -> Int32
```

Absolute value. Wraps at the minimum value
(`Int32.minValue.abs() == Int32.minValue`); use
`absChecked` if that's a problem.

_Defined in `lang/std/num/int32.ks`._

### Implements `Steppable`

#### function `predecessor`

```kestrel
public func predecessor() -> Int32
```

Predecessor — `self - 1`. Wraps at `minValue`.

_Defined in `lang/std/num/int32.ks`._

#### function `successor`

```kestrel
public func successor() -> Int32
```

Successor — `self + 1`. Wraps at `maxValue`. Used by `for-in` over
integer ranges.

_Defined in `lang/std/num/int32.ks`._

### Implements `Comparable`

#### function `compare`

```kestrel
public func compare(Int32) -> Ordering
```

Three-way comparison returning an `Ordering`. Signed types compare
using two's-complement ordering; unsigned types use natural ordering.

##### Examples

```
(1).compare(other: 2);   // .Less
(2).compare(other: 2);   // .Equal
(3).compare(other: 2);   // .Greater
```

_Defined in `lang/std/num/int32.ks`._

### Implements `Equatable`

#### function `equals`

```kestrel
public func equals(Int32) -> Bool
```

Bit-for-bit equality. Backs the `==` operator.

##### Examples

```
(42).equals(other: 42);  // true
42 == 42;                // true
```

_Defined in `lang/std/num/int32.ks`._

### Implements `Matchable`

#### function `matches`

```kestrel
public func matches(Int32) -> Bool
```

Pattern-matching hook for `Matchable`. Identical to `equals`.

_Defined in `lang/std/num/int32.ks`._

### Implements `Formattable`

#### function `format`

```kestrel
public func format(FormatOptions) -> String
```

Renders the integer to a `String`, honouring the supplied
`FormatOptions`. Implements the `Formattable` protocol.

Recognised options:
- `radix` — base in `[2, 36]`; out-of-range values fall back to 10.
- `width` — minimum output width; shorter values are padded.
- `fill` / `alignment` — padding character and side.
- `sign` — `.Negative` (default), `.Always`, or `.Space`.
- `uppercase` — uppercase hex digits.
- `alternate` — emit the `0b` / `0o` / `0x` prefix.

##### Examples

```
(42).format();                                           // "42"
(255).format(options: .{radix: 16});                     // "ff"
(255).format(options: .{radix: 16, uppercase: true});    // "FF"
(255).format(options: .{radix: 16, alternate: true});    // "0xff"
(42).format(options: .{radix: 2, alternate: true});      // "0b101010"
(42).format(options: .{width: .Some(5), fill: '0'});     // "00042"
(-42).format(options: .{sign: .Always});                 // "-42"
```

_Defined in `lang/std/num/int32.ks`._

### Implements `Hash`

#### function `hash`

```kestrel
public func hash[H](into: mutating H) where H: Hasher
```

Feeds the raw bytes of this value into `hasher`. Endianness-agnostic
only within a single process — do not persist hashes across builds.

_Defined in `lang/std/num/int32.ks`._

### Implements `Addable`

#### typealias `Output`

```kestrel
type Output = Int32
```

_Defined in `lang/std/num/int32.ks`._

#### typealias `Output`

```kestrel
type Output = Int32
```

_Defined in `lang/std/num/int32.ks`._

#### typealias `Output`

```kestrel
type Output = Int32
```

_Defined in `lang/std/num/int32.ks`._

#### typealias `Output`

```kestrel
type Output = Int32
```

_Defined in `lang/std/num/int32.ks`._

#### typealias `Output`

```kestrel
type Output = Int32
```

_Defined in `lang/std/num/int32.ks`._

#### typealias `Output`

```kestrel
type Output = Int32
```

_Defined in `lang/std/num/int32.ks`._

#### typealias `Output`

```kestrel
type Output = Int32
```

_Defined in `lang/std/num/int32.ks`._

#### typealias `Output`

```kestrel
type Output = Int32
```

_Defined in `lang/std/num/int32.ks`._

#### typealias `Output`

```kestrel
type Output = Int32
```

_Defined in `lang/std/num/int32.ks`._

#### typealias `Output`

```kestrel
type Output = Int32
```

_Defined in `lang/std/num/int32.ks`._

#### typealias `Output`

```kestrel
type Output = Int32
```

_Defined in `lang/std/num/int32.ks`._

#### typealias `Output`

```kestrel
type Output = Int32
```

_Defined in `lang/std/num/int32.ks`._

#### typealias `Output`

```kestrel
type Output = Range[Int32]
```

_Defined in `lang/std/num/int32.ks`._

#### typealias `Output`

```kestrel
type Output = ClosedRange[Int32]
```

_Defined in `lang/std/num/int32.ks`._

#### function `add`

```kestrel
public func add(Int32) -> Int32
```

`self + other`, wrapping on overflow. Use `addChecked` to detect or
`addSaturating` to clamp.

_Defined in `lang/std/num/int32.ks`._

#### field `zero`

```kestrel
public static var zero: Int32 { get }
```

The additive identity, `0`.

_Defined in `lang/std/num/int32.ks`._

### Implements `Subtractable`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/arithmetic.ks`._

#### function `subtract`

```kestrel
public func subtract(Int32) -> Int32
```

`self - other`, wrapping on overflow.

_Defined in `lang/std/num/int32.ks`._

### Implements `Multipliable`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/arithmetic.ks`._

#### function `multiply`

```kestrel
public func multiply(Int32) -> Int32
```

`self * other`, wrapping on overflow.

_Defined in `lang/std/num/int32.ks`._

#### field `one`

```kestrel
public static var one: Int32 { get }
```

The multiplicative identity, `1`.

_Defined in `lang/std/num/int32.ks`._

### Implements `Divisible`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/arithmetic.ks`._

#### function `divide`

```kestrel
public func divide(Int32) -> Int32
```

Truncating integer division (`self / other`). For signed types,
`minValue / -1` wraps; use `divideChecked` to detect.

##### Errors

Traps on division by zero (LLVM `udiv`/`sdiv` are UB on zero — the
process aborts before producing a result).

_Defined in `lang/std/num/int32.ks`._

### Implements `Modulo`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/arithmetic.ks`._

#### function `modulo`

```kestrel
public func modulo(Int32) -> Int32
```

`self % other` — truncated remainder; the result has the sign of
`self` for signed types.

##### Errors

Traps on division by zero, like `divide`.

_Defined in `lang/std/num/int32.ks`._

### Implements `Negatable`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/arithmetic.ks`._

#### function `negate`

```kestrel
public func negate() -> Int32
```

Two's-complement negation. Wraps at the minimum value:
`Int32.minValue.negate() == Int32.minValue`. Use
`negateChecked` to surface the overflow.

_Defined in `lang/std/num/int32.ks`._

### Implements `BitwiseAnd`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/bitwise.ks`._

#### function `bitwiseAnd`

```kestrel
public func bitwiseAnd(Int32) -> Int32
```

Bitwise AND. `0b1010 & 0b1100 == 0b1000`.

_Defined in `lang/std/num/int32.ks`._

### Implements `BitwiseOr`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/bitwise.ks`._

#### function `bitwiseOr`

```kestrel
public func bitwiseOr(Int32) -> Int32
```

Bitwise OR. `0b1010 | 0b1100 == 0b1110`.

_Defined in `lang/std/num/int32.ks`._

### Implements `BitwiseXor`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/bitwise.ks`._

#### function `bitwiseXor`

```kestrel
public func bitwiseXor(Int32) -> Int32
```

Bitwise XOR. `0b1010 ^ 0b1100 == 0b0110`.

_Defined in `lang/std/num/int32.ks`._

### Implements `BitwiseNot`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/bitwise.ks`._

#### function `bitwiseNot`

```kestrel
public func bitwiseNot() -> Int32
```

Bitwise NOT — flips all bits. For signed types this is `-self - 1`.

_Defined in `lang/std/num/int32.ks`._

### Implements `LeftShift`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/bitwise.ks`._

#### function `shiftLeft`

```kestrel
public func shiftLeft(by: lang.i64) -> Int32
```

Left shift by `count`. Behavior is undefined when `count >= bitWidth`
— pre-mask the count if you can't guarantee the bound.

_Defined in `lang/std/num/int32.ks`._

### Implements `RightShift`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/bitwise.ks`._

#### function `shiftRight`

```kestrel
public func shiftRight(by: lang.i64) -> Int32
```

Right shift by `count`. Arithmetic (sign-extending) for signed types,
logical (zero-filling) for unsigned. Same `count` precondition as
`shiftLeft`.

_Defined in `lang/std/num/int32.ks`._

### Implements `AddAssign`

#### function `addAssign`

```kestrel
public mutating func addAssign(Int32)
```

`self += other`

_Defined in `lang/std/num/int32.ks`._

### Implements `SubtractAssign`

#### function `subtractAssign`

```kestrel
public mutating func subtractAssign(Int32)
```

`self -= other`

_Defined in `lang/std/num/int32.ks`._

### Implements `MultiplyAssign`

#### function `multiplyAssign`

```kestrel
public mutating func multiplyAssign(Int32)
```

`self *= other`

_Defined in `lang/std/num/int32.ks`._

### Implements `DivideAssign`

#### function `divideAssign`

```kestrel
public mutating func divideAssign(Int32)
```

`self /= other`

_Defined in `lang/std/num/int32.ks`._

### Implements `ModuloAssign`

#### function `modAssign`

```kestrel
public mutating func modAssign(Int32)
```

`self %= other`

_Defined in `lang/std/num/int32.ks`._

### Implements `BitwiseAndAssign`

#### function `bitwiseAndAssign`

```kestrel
public mutating func bitwiseAndAssign(Int32)
```

`self &= other`

_Defined in `lang/std/num/int32.ks`._

### Implements `BitwiseOrAssign`

#### function `bitwiseOrAssign`

```kestrel
public mutating func bitwiseOrAssign(Int32)
```

`self |= other`

_Defined in `lang/std/num/int32.ks`._

### Implements `BitwiseXorAssign`

#### function `bitwiseXorAssign`

```kestrel
public mutating func bitwiseXorAssign(Int32)
```

`self ^= other`

_Defined in `lang/std/num/int32.ks`._

### Implements `LeftShiftAssign`

#### function `shiftLeftAssign`

```kestrel
public mutating func shiftLeftAssign(by: lang.i64)
```

`self <<= count`

_Defined in `lang/std/num/int32.ks`._

### Implements `RightShiftAssign`

#### function `shiftRightAssign`

```kestrel
public mutating func shiftRightAssign(by: lang.i64)
```

`self >>= count`

_Defined in `lang/std/num/int32.ks`._

### Implements `ExpressibleByIntLiteral`

#### initializer `Int Literal`

```kestrel
init(intLiteral: lang.i64)
```

Builds an instance from an integer literal.

_Defined in `lang/std/core/literals.ks`._

### Implements `Defaultable`

#### initializer `Default`

```kestrel
init()
```

Builds the default-valued instance.

_Defined in `lang/std/core/protocols.ks`._

### Implements `RangeConstructible`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/range.ks`._

#### function `exclusiveRange`

```kestrel
public func exclusiveRange(to: Int32) -> Range[Int32]
```

Builds a half-open range `self..<end`. Sugar for the `..<` operator.

_Defined in `lang/std/num/int32.ks`._

### Implements `ClosedRangeConstructible`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/range.ks`._

#### function `inclusiveRange`

```kestrel
public func inclusiveRange(to: Int32) -> ClosedRange[Int32]
```

Builds a closed range `self..=end`. Sugar for the `..=` operator.

_Defined in `lang/std/num/int32.ks`._

### Implements `Convertible`

#### initializer `From Source`

```kestrel
init(from: From)
```

Creates an instance from `value`.

_Defined in `lang/std/core/convertible.ks`._

## struct `Int64`

```kestrel
public struct Int64 { /* private fields */ }
```

A 64-bit signed integer.

Int64 is the 64-bit member of the integer family. The same surface
area is provided across all widths; switch widths to trade range for memory
or to match an FFI ABI. Arithmetic wraps on overflow by default — use the
`*Checked` variants for overflow detection or `*Saturating` to clamp to
`minValue`/`maxValue`. The type is `FFISafe` and lays out as a single
`lang.i64` so it can cross C boundaries unchanged.

### Examples

```
let a: Int64 = 100;
let b = a + 50;        // 150
let c = a * 2;         // 200
let d = a.addChecked(Int64.maxValue);  // None (overflow detected)
```

```
// Bit twiddling
(0b1010).countOnes      // 2
(1).shiftLeft(by: 4)    // 16
(-1).leadingZeros       // 0  (all bits set)
```

### Representation

A single `lang.i64` field. No padding, no headers — bit-identical
to the corresponding C type.

_Defined in `lang/std/num/int64.ks`._

### Members

#### initializer `Default`

```kestrel
public init()
```

Creates the zero value, satisfying `Defaultable`.

##### Examples

```
let n = Int64();   // 0
```

_Defined in `lang/std/num/int64.ks`._

#### initializer `From Integer`

```kestrel
public init(from: Int8)
```

Converts from `Int8`. Narrowing conversions truncate the high
bits; signed→unsigned reinterprets the bit pattern.

_Defined in `lang/std/num/int64.ks`._

#### initializer `From Integer`

```kestrel
public init(from: Int16)
```

Converts from `Int16`. Narrowing conversions truncate the high
bits; signed→unsigned reinterprets the bit pattern.

_Defined in `lang/std/num/int64.ks`._

#### initializer `From Integer`

```kestrel
public init(from: Int32)
```

Converts from `Int32`. Narrowing conversions truncate the high
bits; signed→unsigned reinterprets the bit pattern.

_Defined in `lang/std/num/int64.ks`._

#### initializer `From Integer`

```kestrel
public init(from: UInt8)
```

Converts from `UInt8`. Narrowing conversions truncate the high
bits; signed→unsigned reinterprets the bit pattern.

_Defined in `lang/std/num/int64.ks`._

#### initializer `From Integer`

```kestrel
public init(from: UInt16)
```

Converts from `UInt16`. Narrowing conversions truncate the high
bits; signed→unsigned reinterprets the bit pattern.

_Defined in `lang/std/num/int64.ks`._

#### initializer `From Integer`

```kestrel
public init(from: UInt32)
```

Converts from `UInt32`. Narrowing conversions truncate the high
bits; signed→unsigned reinterprets the bit pattern.

_Defined in `lang/std/num/int64.ks`._

#### initializer `From Integer`

```kestrel
public init(from: UInt64)
```

Converts from `UInt64`. Narrowing conversions truncate the high
bits; signed→unsigned reinterprets the bit pattern.

_Defined in `lang/std/num/int64.ks`._

#### initializer `From Raw`

```kestrel
init(raw: lang.i64)
```

Wraps an existing `lang.i64` without conversion. Internal
constructor used by intrinsics; not part of the public API.

_Defined in `lang/std/num/int64.ks`._

#### initializer `Int Literal`

```kestrel
public init(intLiteral: lang.i64)
```

Compiler-emitted bridge that turns an integer literal into a Int64.

You will rarely call this directly — write the literal and let the
`ExpressibleByIntLiteral` protocol pick it up. For widths smaller than
64 bits the literal is truncated with `lang.cast_i64_i64`.

##### Examples

```
let n: Int64 = 42;            // implicit
let m = Int64(intLiteral: 42);  // explicit
```

_Defined in `lang/std/num/int64.ks`._

#### function `absChecked`

```kestrel
public func absChecked() -> Int64?
```

Absolute value that returns `None` for `minValue` (whose absolute
value overflows).

_Defined in `lang/std/num/int64.ks`._

#### function `absSaturating`

```kestrel
public func absSaturating() -> Int64
```

Absolute value that returns `maxValue` instead of wrapping `minValue`.

_Defined in `lang/std/num/int64.ks`._

#### function `addChecked`

```kestrel
public func addChecked(Int64) -> Int64?
```

Wrapping addition that returns `None` instead of overflowing.

_Defined in `lang/std/num/int64.ks`._

#### function `addSaturating`

```kestrel
public func addSaturating(Int64) -> Int64
```

Addition that clamps to `maxValue`/`minValue` instead of wrapping.

_Defined in `lang/std/num/int64.ks`._

#### field `bitWidth`

```kestrel
public static var bitWidth: Int64 { get }
```

The width in bits (64). Useful for shift bounds and bit-walks.

_Defined in `lang/std/num/int64.ks`._

#### field `byteSwapped`

```kestrel
public var byteSwapped: Int64 { get }
```

Value with its byte order reversed. Use to convert between big- and
little-endian; lowered to a `bswap` intrinsic.

_Defined in `lang/std/num/int64.ks`._

#### function `clamp`

```kestrel
public func clamp(Int64, Int64) -> Int64
```

Clamps `self` into `[min, max]`. Caller is responsible for ensuring
`min <= max`; otherwise the result is undefined.

##### Examples

```
(5).clamp(min: 0, max: 10);    // 5
(-5).clamp(min: 0, max: 10);   // 0
(15).clamp(min: 0, max: 10);   // 10
```

_Defined in `lang/std/num/int64.ks`._

#### field `countOnes`

```kestrel
public var countOnes: Int64 { get }
```

Population count — the number of `1` bits in the binary representation.

Lowered to a `popcount` intrinsic where the target supports it.

##### Examples

```
(0b1010).countOnes;  // 2
(0b1111).countOnes;  // 4
(0).countOnes;       // 0
```

_Defined in `lang/std/num/int64.ks`._

#### field `countZeros`

```kestrel
public var countZeros: Int64 { get }
```

Complement of `countOnes`: equal to `bitWidth - countOnes`.

_Defined in `lang/std/num/int64.ks`._

#### function `divideChecked`

```kestrel
public func divideChecked(Int64) -> Int64?
```

Division that returns `None` for divide-by-zero or for the
`minValue / -1` overflow case.

_Defined in `lang/std/num/int64.ks`._

#### function `fromBytes`

```kestrel
public static func fromBytes(std.collections.Array[UInt8]) -> Int64?
```

Reassembles a `Int64` from 8 bytes in native (host) byte
order. Returns `None` if the input is not exactly 8 bytes long.

_Defined in `lang/std/num/int64.ks`._

#### function `fromBytesBigEndian`

```kestrel
public static func fromBytesBigEndian(std.collections.Array[UInt8]) -> Int64?
```

Reassembles a `Int64` from 8 bytes in big-endian order.
Returns `None` if the input is not exactly 8 bytes long.

_Defined in `lang/std/num/int64.ks`._

#### function `fromBytesLittleEndian`

```kestrel
public static func fromBytesLittleEndian(std.collections.Array[UInt8]) -> Int64?
```

Reassembles a `Int64` from 8 bytes in little-endian order.
Returns `None` if the input is not exactly 8 bytes long.

_Defined in `lang/std/num/int64.ks`._

#### function `gcd`

```kestrel
public func gcd(Int64) -> Int64
```

Greatest common divisor via Euclidean algorithm. For signed types
the inputs are taken absolute first; the result is always non-negative.

##### Examples

```
(12).gcd(8);   // 4
(17).gcd(5);   // 1   (coprime)
(-12).gcd(8);  // 4
```

_Defined in `lang/std/num/int64.ks`._

#### field `isNegative`

```kestrel
public var isNegative: Bool { get }
```

True when `self < 0`.

_Defined in `lang/std/num/int64.ks`._

#### field `isPositive`

```kestrel
public var isPositive: Bool { get }
```

True when `self > 0`.

_Defined in `lang/std/num/int64.ks`._

#### field `isPowerOfTwo`

```kestrel
public var isPowerOfTwo: Bool { get }
```

True when the value is a positive power of two (`2^k` for `k >= 0`).

Zero and negatives are excluded. Cheap branchless test built on
`x & (x - 1) == 0`.

##### Examples

```
(1).isPowerOfTwo;   // true  (2^0)
(4).isPowerOfTwo;   // true  (2^2)
(3).isPowerOfTwo;   // false
(0).isPowerOfTwo;   // false
```

_Defined in `lang/std/num/int64.ks`._

#### field `isZero`

```kestrel
public var isZero: Bool { get }
```

True when `self == 0`.

_Defined in `lang/std/num/int64.ks`._

#### function `lcm`

```kestrel
public func lcm(Int64) -> Int64
```

Least common multiple, computed as `|self| / gcd(self, other) * |other|`
to avoid intermediate overflow. Returns zero if either input is zero.

##### Examples

```
(4).lcm(6);   // 12
(3).lcm(5);   // 15
(0).lcm(7);   // 0
```

_Defined in `lang/std/num/int64.ks`._

#### field `leadingZeros`

```kestrel
public var leadingZeros: Int64 { get }
```

Number of leading zero bits, counting from the most-significant end.

For zero, returns `bitWidth`.

##### Examples

```
(1).leadingZeros;   // bitWidth - 1
(0).leadingZeros;   // bitWidth
```

_Defined in `lang/std/num/int64.ks`._

#### field `maxValue`

```kestrel
public static var maxValue: Int64 { get }
```

The largest representable value.
This is 2^63 - 1 (9_223_372_036_854_775_807).

_Defined in `lang/std/num/int64.ks`._

#### field `minValue`

```kestrel
public static var minValue: Int64 { get }
```

The smallest representable value.
This is -2^63 (-9_223_372_036_854_775_808).
Note that for signed types `minValue.negate()` overflows back to
itself; use `negateChecked()` if you need to detect that.

_Defined in `lang/std/num/int64.ks`._

#### function `multiplyChecked`

```kestrel
public func multiplyChecked(Int64) -> Int64?
```

Wrapping multiplication that returns `None` instead of overflowing.
Implemented by multiplying then dividing back; replace with an
overflow-detecting intrinsic when one is available.

_Defined in `lang/std/num/int64.ks`._

#### function `multiplySaturating`

```kestrel
public func multiplySaturating(Int64) -> Int64
```

Multiplication that clamps to `maxValue`/`minValue` instead of wrapping.
The clamp direction follows the algebraic sign of the would-be result.

_Defined in `lang/std/num/int64.ks`._

#### function `negateChecked`

```kestrel
public func negateChecked() -> Int64?
```

Negation that returns `None` for `minValue` (whose negation overflows).

_Defined in `lang/std/num/int64.ks`._

#### function `negateSaturating`

```kestrel
public func negateSaturating() -> Int64
```

Negation that returns `maxValue` instead of wrapping `minValue`.

_Defined in `lang/std/num/int64.ks`._

#### function `parse`

```kestrel
public static func parse(String) -> Int64?
```

Parses a base-10 integer literal, optionally prefixed with `+` or
`-`. Returns `None` for an empty string, a non-digit character,
or a value that does not fit in `Int64`.

##### Examples

```
Int64.parse(string: "42");    // Some(42)
Int64.parse(string: "-7");    // Some(-7)
Int64.parse(string: "abc");   // None
Int64.parse(string: "");      // None
```

_Defined in `lang/std/num/int64.ks`._

#### function `parse`

```kestrel
public static func parse(String, Int64) -> Int64?
```

Parses an integer in `radix` (base 2–36 inclusive). Letters a–z are
case-insensitive and represent digit values 10–35. Returns `None`
for an out-of-range radix, an empty string, an unrecognised digit,
or a value that overflows `Int64`.

##### Examples

```
Int64.parse(string: "ff", radix: 16);     // Some(255 if it fits, else None)
Int64.parse(string: "101010", radix: 2);  // Some(42)
Int64.parse(string: "z", radix: 36);      // Some(35)
```

_Defined in `lang/std/num/int64.ks`._

#### function `pow`

```kestrel
public func pow(Int64) -> Int64
```

Raises `self` to `exponent` via binary exponentiation. Wraps on
overflow. Negative exponents return zero (integer truncation of
the would-be fraction).

##### Examples

```
(2).pow(10);  // 1024
(3).pow(4);   // 81
(5).pow(-1);  // 0
```

_Defined in `lang/std/num/int64.ks`._

#### field `raw`

```kestrel
public var raw: lang.i64
```

The underlying primitive `lang.i64` value. Exposed for FFI
and intrinsic use; prefer the typed surface for everything else.

_Defined in `lang/std/num/int64.ks`._

#### function `rotateLeft`

```kestrel
public func rotateLeft(by: Int64) -> Int64
```

Rotates bits left by `count`, modulo `bitWidth`. Bits shifted past the
MSB re-enter at the LSB.

_Defined in `lang/std/num/int64.ks`._

#### function `rotateRight`

```kestrel
public func rotateRight(by: Int64) -> Int64
```

Rotates bits right by `count`, modulo `bitWidth`. Mirror of
`rotateLeft`.

_Defined in `lang/std/num/int64.ks`._

#### field `sign`

```kestrel
public var sign: Int64 { get }
```

Sign as a `Int64`: `-1`, `0`, or `1`.

_Defined in `lang/std/num/int64.ks`._

#### function `subtractChecked`

```kestrel
public func subtractChecked(Int64) -> Int64?
```

Wrapping subtraction that returns `None` instead of overflowing.

_Defined in `lang/std/num/int64.ks`._

#### function `subtractSaturating`

```kestrel
public func subtractSaturating(Int64) -> Int64
```

Subtraction that clamps to `maxValue`/`minValue` instead of wrapping.

_Defined in `lang/std/num/int64.ks`._

#### function `toBytes`

```kestrel
public func toBytes() -> std.collections.Array[UInt8]
```

Splits this integer into 8 bytes in *native* (host) byte order.
Use `toBytesBigEndian` / `toBytesLittleEndian` when serialising for
a fixed wire format.

##### Examples

```
let bytes = Int64.maxValue.toBytes();   // 8 bytes, host order
```

_Defined in `lang/std/num/int64.ks`._

#### function `toBytesBigEndian`

```kestrel
public func toBytesBigEndian() -> std.collections.Array[UInt8]
```

Splits this integer into 8 bytes in big-endian order (most
significant byte first — i.e. network byte order).

_Defined in `lang/std/num/int64.ks`._

#### function `toBytesLittleEndian`

```kestrel
public func toBytesLittleEndian() -> std.collections.Array[UInt8]
```

Splits this integer into 8 bytes in little-endian order (least
significant byte first).

_Defined in `lang/std/num/int64.ks`._

#### field `trailingZeros`

```kestrel
public var trailingZeros: Int64 { get }
```

Number of trailing zero bits. Equal to `log2(self & -self)` for non-zero
values; returns `bitWidth` for zero. Useful for finding the largest
power of two dividing the value.

_Defined in `lang/std/num/int64.ks`._

### Implements `SignedInteger`

#### function `abs`

```kestrel
public func abs() -> Int64
```

Absolute value. Wraps at the minimum value
(`Int64.minValue.abs() == Int64.minValue`); use
`absChecked` if that's a problem.

_Defined in `lang/std/num/int64.ks`._

### Implements `Steppable`

#### function `predecessor`

```kestrel
public func predecessor() -> Int64
```

Predecessor — `self - 1`. Wraps at `minValue`.

_Defined in `lang/std/num/int64.ks`._

#### function `successor`

```kestrel
public func successor() -> Int64
```

Successor — `self + 1`. Wraps at `maxValue`. Used by `for-in` over
integer ranges.

_Defined in `lang/std/num/int64.ks`._

### Implements `Comparable`

#### function `compare`

```kestrel
public func compare(Int64) -> Ordering
```

Three-way comparison returning an `Ordering`. Signed types compare
using two's-complement ordering; unsigned types use natural ordering.

##### Examples

```
(1).compare(other: 2);   // .Less
(2).compare(other: 2);   // .Equal
(3).compare(other: 2);   // .Greater
```

_Defined in `lang/std/num/int64.ks`._

### Implements `Equatable`

#### function `equals`

```kestrel
public func equals(Int64) -> Bool
```

Bit-for-bit equality. Backs the `==` operator.

##### Examples

```
(42).equals(other: 42);  // true
42 == 42;                // true
```

_Defined in `lang/std/num/int64.ks`._

### Implements `Matchable`

#### function `matches`

```kestrel
public func matches(Int64) -> Bool
```

Pattern-matching hook for `Matchable`. Identical to `equals`.

_Defined in `lang/std/num/int64.ks`._

### Implements `Formattable`

#### function `format`

```kestrel
public func format(FormatOptions) -> String
```

Renders the integer to a `String`, honouring the supplied
`FormatOptions`. Implements the `Formattable` protocol.

Recognised options:
- `radix` — base in `[2, 36]`; out-of-range values fall back to 10.
- `width` — minimum output width; shorter values are padded.
- `fill` / `alignment` — padding character and side.
- `sign` — `.Negative` (default), `.Always`, or `.Space`.
- `uppercase` — uppercase hex digits.
- `alternate` — emit the `0b` / `0o` / `0x` prefix.

##### Examples

```
(42).format();                                           // "42"
(255).format(options: .{radix: 16});                     // "ff"
(255).format(options: .{radix: 16, uppercase: true});    // "FF"
(255).format(options: .{radix: 16, alternate: true});    // "0xff"
(42).format(options: .{radix: 2, alternate: true});      // "0b101010"
(42).format(options: .{width: .Some(5), fill: '0'});     // "00042"
(-42).format(options: .{sign: .Always});                 // "-42"
```

_Defined in `lang/std/num/int64.ks`._

### Implements `Hash`

#### function `hash`

```kestrel
public func hash[H](into: mutating H) where H: Hasher
```

Feeds the raw bytes of this value into `hasher`. Endianness-agnostic
only within a single process — do not persist hashes across builds.

_Defined in `lang/std/num/int64.ks`._

### Implements `Addable`

#### typealias `Output`

```kestrel
type Output = Int64
```

_Defined in `lang/std/num/int64.ks`._

#### typealias `Output`

```kestrel
type Output = Int64
```

_Defined in `lang/std/num/int64.ks`._

#### typealias `Output`

```kestrel
type Output = Int64
```

_Defined in `lang/std/num/int64.ks`._

#### typealias `Output`

```kestrel
type Output = Int64
```

_Defined in `lang/std/num/int64.ks`._

#### typealias `Output`

```kestrel
type Output = Int64
```

_Defined in `lang/std/num/int64.ks`._

#### typealias `Output`

```kestrel
type Output = Int64
```

_Defined in `lang/std/num/int64.ks`._

#### typealias `Output`

```kestrel
type Output = Int64
```

_Defined in `lang/std/num/int64.ks`._

#### typealias `Output`

```kestrel
type Output = Int64
```

_Defined in `lang/std/num/int64.ks`._

#### typealias `Output`

```kestrel
type Output = Int64
```

_Defined in `lang/std/num/int64.ks`._

#### typealias `Output`

```kestrel
type Output = Int64
```

_Defined in `lang/std/num/int64.ks`._

#### typealias `Output`

```kestrel
type Output = Int64
```

_Defined in `lang/std/num/int64.ks`._

#### typealias `Output`

```kestrel
type Output = Int64
```

_Defined in `lang/std/num/int64.ks`._

#### typealias `Output`

```kestrel
type Output = Range[Int64]
```

_Defined in `lang/std/num/int64.ks`._

#### typealias `Output`

```kestrel
type Output = ClosedRange[Int64]
```

_Defined in `lang/std/num/int64.ks`._

#### function `add`

```kestrel
public func add(Int64) -> Int64
```

`self + other`, wrapping on overflow. Use `addChecked` to detect or
`addSaturating` to clamp.

_Defined in `lang/std/num/int64.ks`._

#### field `zero`

```kestrel
public static var zero: Int64 { get }
```

The additive identity, `0`.

_Defined in `lang/std/num/int64.ks`._

### Implements `Subtractable`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/arithmetic.ks`._

#### function `subtract`

```kestrel
public func subtract(Int64) -> Int64
```

`self - other`, wrapping on overflow.

_Defined in `lang/std/num/int64.ks`._

### Implements `Multipliable`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/arithmetic.ks`._

#### function `multiply`

```kestrel
public func multiply(Int64) -> Int64
```

`self * other`, wrapping on overflow.

_Defined in `lang/std/num/int64.ks`._

#### field `one`

```kestrel
public static var one: Int64 { get }
```

The multiplicative identity, `1`.

_Defined in `lang/std/num/int64.ks`._

### Implements `Divisible`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/arithmetic.ks`._

#### function `divide`

```kestrel
public func divide(Int64) -> Int64
```

Truncating integer division (`self / other`). For signed types,
`minValue / -1` wraps; use `divideChecked` to detect.

##### Errors

Traps on division by zero (LLVM `udiv`/`sdiv` are UB on zero — the
process aborts before producing a result).

_Defined in `lang/std/num/int64.ks`._

### Implements `Modulo`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/arithmetic.ks`._

#### function `modulo`

```kestrel
public func modulo(Int64) -> Int64
```

`self % other` — truncated remainder; the result has the sign of
`self` for signed types.

##### Errors

Traps on division by zero, like `divide`.

_Defined in `lang/std/num/int64.ks`._

### Implements `Negatable`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/arithmetic.ks`._

#### function `negate`

```kestrel
public func negate() -> Int64
```

Two's-complement negation. Wraps at the minimum value:
`Int64.minValue.negate() == Int64.minValue`. Use
`negateChecked` to surface the overflow.

_Defined in `lang/std/num/int64.ks`._

### Implements `BitwiseAnd`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/bitwise.ks`._

#### function `bitwiseAnd`

```kestrel
public func bitwiseAnd(Int64) -> Int64
```

Bitwise AND. `0b1010 & 0b1100 == 0b1000`.

_Defined in `lang/std/num/int64.ks`._

### Implements `BitwiseOr`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/bitwise.ks`._

#### function `bitwiseOr`

```kestrel
public func bitwiseOr(Int64) -> Int64
```

Bitwise OR. `0b1010 | 0b1100 == 0b1110`.

_Defined in `lang/std/num/int64.ks`._

### Implements `BitwiseXor`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/bitwise.ks`._

#### function `bitwiseXor`

```kestrel
public func bitwiseXor(Int64) -> Int64
```

Bitwise XOR. `0b1010 ^ 0b1100 == 0b0110`.

_Defined in `lang/std/num/int64.ks`._

### Implements `BitwiseNot`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/bitwise.ks`._

#### function `bitwiseNot`

```kestrel
public func bitwiseNot() -> Int64
```

Bitwise NOT — flips all bits. For signed types this is `-self - 1`.

_Defined in `lang/std/num/int64.ks`._

### Implements `LeftShift`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/bitwise.ks`._

#### function `shiftLeft`

```kestrel
public func shiftLeft(by: lang.i64) -> Int64
```

Left shift by `count`. Behavior is undefined when `count >= bitWidth`
— pre-mask the count if you can't guarantee the bound.

_Defined in `lang/std/num/int64.ks`._

### Implements `RightShift`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/bitwise.ks`._

#### function `shiftRight`

```kestrel
public func shiftRight(by: lang.i64) -> Int64
```

Right shift by `count`. Arithmetic (sign-extending) for signed types,
logical (zero-filling) for unsigned. Same `count` precondition as
`shiftLeft`.

_Defined in `lang/std/num/int64.ks`._

### Implements `AddAssign`

#### function `addAssign`

```kestrel
public mutating func addAssign(Int64)
```

`self += other`

_Defined in `lang/std/num/int64.ks`._

### Implements `SubtractAssign`

#### function `subtractAssign`

```kestrel
public mutating func subtractAssign(Int64)
```

`self -= other`

_Defined in `lang/std/num/int64.ks`._

### Implements `MultiplyAssign`

#### function `multiplyAssign`

```kestrel
public mutating func multiplyAssign(Int64)
```

`self *= other`

_Defined in `lang/std/num/int64.ks`._

### Implements `DivideAssign`

#### function `divideAssign`

```kestrel
public mutating func divideAssign(Int64)
```

`self /= other`

_Defined in `lang/std/num/int64.ks`._

### Implements `ModuloAssign`

#### function `modAssign`

```kestrel
public mutating func modAssign(Int64)
```

`self %= other`

_Defined in `lang/std/num/int64.ks`._

### Implements `BitwiseAndAssign`

#### function `bitwiseAndAssign`

```kestrel
public mutating func bitwiseAndAssign(Int64)
```

`self &= other`

_Defined in `lang/std/num/int64.ks`._

### Implements `BitwiseOrAssign`

#### function `bitwiseOrAssign`

```kestrel
public mutating func bitwiseOrAssign(Int64)
```

`self |= other`

_Defined in `lang/std/num/int64.ks`._

### Implements `BitwiseXorAssign`

#### function `bitwiseXorAssign`

```kestrel
public mutating func bitwiseXorAssign(Int64)
```

`self ^= other`

_Defined in `lang/std/num/int64.ks`._

### Implements `LeftShiftAssign`

#### function `shiftLeftAssign`

```kestrel
public mutating func shiftLeftAssign(by: lang.i64)
```

`self <<= count`

_Defined in `lang/std/num/int64.ks`._

### Implements `RightShiftAssign`

#### function `shiftRightAssign`

```kestrel
public mutating func shiftRightAssign(by: lang.i64)
```

`self >>= count`

_Defined in `lang/std/num/int64.ks`._

### Implements `ExpressibleByIntLiteral`

#### initializer `Int Literal`

```kestrel
init(intLiteral: lang.i64)
```

Builds an instance from an integer literal.

_Defined in `lang/std/core/literals.ks`._

### Implements `Defaultable`

#### initializer `Default`

```kestrel
init()
```

Builds the default-valued instance.

_Defined in `lang/std/core/protocols.ks`._

### Implements `RangeConstructible`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/range.ks`._

#### function `exclusiveRange`

```kestrel
public func exclusiveRange(to: Int64) -> Range[Int64]
```

Builds a half-open range `self..<end`. Sugar for the `..<` operator.

_Defined in `lang/std/num/int64.ks`._

### Implements `ClosedRangeConstructible`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/range.ks`._

#### function `inclusiveRange`

```kestrel
public func inclusiveRange(to: Int64) -> ClosedRange[Int64]
```

Builds a closed range `self..=end`. Sugar for the `..=` operator.

_Defined in `lang/std/num/int64.ks`._

### Implements `Convertible`

#### initializer `From Source`

```kestrel
init(from: From)
```

Creates an instance from `value`.

_Defined in `lang/std/core/convertible.ks`._

### Implements `ArrayIndex`

#### typealias `ArrayYield`

```kestrel
type ArrayYield = T
```

_Defined in `lang/std/collections/array.ks`._

#### function `readArray`

```kestrel
public func readArray(from: Array[T]) -> T
```

_Defined in `lang/std/collections/array.ks`._

#### function `readArrayChecked`

```kestrel
public func readArrayChecked(from: Array[T]) -> T?
```

_Defined in `lang/std/collections/array.ks`._

#### function `readArrayUnchecked`

```kestrel
public func readArrayUnchecked(from: Array[T]) -> T
```

_Defined in `lang/std/collections/array.ks`._

#### function `writeArray`

```kestrel
public func writeArray(to: mutating Array[T], value: T)
```

_Defined in `lang/std/collections/array.ks`._

#### function `writeArrayUnchecked`

```kestrel
public func writeArrayUnchecked(to: mutating Array[T], value: T)
```

_Defined in `lang/std/collections/array.ks`._

### Implements `ArrayClampable`

#### typealias `ArrayClampedYield`

```kestrel
type ArrayClampedYield = T?
```

_Defined in `lang/std/collections/array.ks`._

#### function `readArrayClamped`

```kestrel
public func readArrayClamped(from: Array[T]) -> T?
```

_Defined in `lang/std/collections/array.ks`._

#### function `writeArrayClamped`

```kestrel
public func writeArrayClamped(to: mutating Array[T], value: T?)
```

_Defined in `lang/std/collections/array.ks`._

### Implements `ArrayWrappable`

#### typealias `ArrayWrappedYield`

```kestrel
type ArrayWrappedYield = T?
```

_Defined in `lang/std/collections/array.ks`._

#### function `readArrayWrapped`

```kestrel
public func readArrayWrapped(from: Array[T]) -> T?
```

_Defined in `lang/std/collections/array.ks`._

#### function `writeArrayWrapped`

```kestrel
public func writeArrayWrapped(to: mutating Array[T], value: T?)
```

_Defined in `lang/std/collections/array.ks`._

### Implements `SliceIndex`

#### typealias `SliceYield`

```kestrel
type SliceYield = T
```

_Defined in `lang/std/memory/pointer.ks`._

#### function `readSlice`

```kestrel
public func readSlice(from: Slice[T]) -> T
```

_Defined in `lang/std/memory/pointer.ks`._

#### function `readSliceChecked`

```kestrel
public func readSliceChecked(from: Slice[T]) -> T?
```

_Defined in `lang/std/memory/pointer.ks`._

#### function `readSliceUnchecked`

```kestrel
public func readSliceUnchecked(from: Slice[T]) -> T
```

_Defined in `lang/std/memory/pointer.ks`._

#### function `writeSlice`

```kestrel
public func writeSlice(to: Slice[T], value: T)
```

_Defined in `lang/std/memory/pointer.ks`._

#### function `writeSliceUnchecked`

```kestrel
public func writeSliceUnchecked(to: Slice[T], value: T)
```

_Defined in `lang/std/memory/pointer.ks`._

### Implements `SliceClampable`

#### typealias `SliceClampedYield`

```kestrel
type SliceClampedYield = T?
```

_Defined in `lang/std/memory/pointer.ks`._

#### function `readSliceClamped`

```kestrel
public func readSliceClamped(from: Slice[T]) -> T?
```

_Defined in `lang/std/memory/pointer.ks`._

#### function `writeSliceClamped`

```kestrel
public func writeSliceClamped(to: Slice[T], value: T?)
```

_Defined in `lang/std/memory/pointer.ks`._

### Implements `SliceWrappable`

#### typealias `SliceWrappedYield`

```kestrel
type SliceWrappedYield = T?
```

_Defined in `lang/std/memory/pointer.ks`._

#### function `readSliceWrapped`

```kestrel
public func readSliceWrapped(from: Slice[T]) -> T?
```

_Defined in `lang/std/memory/pointer.ks`._

#### function `writeSliceWrapped`

```kestrel
public func writeSliceWrapped(to: Slice[T], value: T?)
```

_Defined in `lang/std/memory/pointer.ks`._

### Implements `BytesIndex`

#### typealias `BytesYield`

```kestrel
type BytesYield = UInt8
```

_Defined in `lang/std/text/views.ks`._

#### function `readBytes`

```kestrel
public func readBytes(from: BytesView) -> UInt8
```

_Defined in `lang/std/text/views.ks`._

#### function `readBytesChecked`

```kestrel
public func readBytesChecked(from: BytesView) -> UInt8?
```

_Defined in `lang/std/text/views.ks`._

#### function `readBytesUnchecked`

```kestrel
public func readBytesUnchecked(from: BytesView) -> UInt8
```

_Defined in `lang/std/text/views.ks`._

### Implements `BytesClampable`

#### typealias `BytesClampedYield`

```kestrel
type BytesClampedYield = UInt8?
```

_Defined in `lang/std/text/views.ks`._

#### function `readBytesClamped`

```kestrel
public func readBytesClamped(from: BytesView) -> UInt8?
```

_Defined in `lang/std/text/views.ks`._

### Implements `CharsIndex`

#### typealias `CharsYield`

```kestrel
type CharsYield = Char
```

_Defined in `lang/std/text/views.ks`._

#### function `readChars`

```kestrel
public func readChars(from: CharsView) -> Char
```

_Defined in `lang/std/text/views.ks`._

#### function `readCharsChecked`

```kestrel
public func readCharsChecked(from: CharsView) -> Char?
```

_Defined in `lang/std/text/views.ks`._

### Implements `CharsClampable`

#### typealias `CharsClampedYield`

```kestrel
type CharsClampedYield = Char?
```

_Defined in `lang/std/text/views.ks`._

#### function `readCharsClamped`

```kestrel
public func readCharsClamped(from: CharsView) -> Char?
```

_Defined in `lang/std/text/views.ks`._

### Implements `GraphemesIndex`

#### typealias `GraphemesYield`

```kestrel
type GraphemesYield = Grapheme
```

_Defined in `lang/std/text/views.ks`._

#### function `readGraphemes`

```kestrel
public func readGraphemes(from: GraphemesView) -> Grapheme
```

_Defined in `lang/std/text/views.ks`._

#### function `readGraphemesChecked`

```kestrel
public func readGraphemesChecked(from: GraphemesView) -> Grapheme?
```

_Defined in `lang/std/text/views.ks`._

### Implements `GraphemesClampable`

#### typealias `GraphemesClampedYield`

```kestrel
type GraphemesClampedYield = Grapheme?
```

_Defined in `lang/std/text/views.ks`._

#### function `readGraphemesClamped`

```kestrel
public func readGraphemesClamped(from: GraphemesView) -> Grapheme?
```

_Defined in `lang/std/text/views.ks`._

### Implements `LinesIndex`

#### typealias `LinesYield`

```kestrel
type LinesYield = String
```

_Defined in `lang/std/text/views.ks`._

#### function `readLines`

```kestrel
public func readLines(from: LinesView) -> String
```

_Defined in `lang/std/text/views.ks`._

#### function `readLinesChecked`

```kestrel
public func readLinesChecked(from: LinesView) -> String?
```

_Defined in `lang/std/text/views.ks`._

### Implements `LinesClampable`

#### typealias `LinesClampedYield`

```kestrel
type LinesClampedYield = String?
```

_Defined in `lang/std/text/views.ks`._

#### function `readLinesClamped`

```kestrel
public func readLinesClamped(from: LinesView) -> String?
```

_Defined in `lang/std/text/views.ks`._

## struct `Int8`

```kestrel
public struct Int8 { /* private fields */ }
```

A 8-bit signed integer.

Int8 is the 8-bit member of the integer family. The same surface
area is provided across all widths; switch widths to trade range for memory
or to match an FFI ABI. Arithmetic wraps on overflow by default — use the
`*Checked` variants for overflow detection or `*Saturating` to clamp to
`minValue`/`maxValue`. The type is `FFISafe` and lays out as a single
`lang.i8` so it can cross C boundaries unchanged.

### Examples

```
let a: Int64 = 100;
let b = a + 50;        // 150
let c = a * 2;         // 200
let d = a.addChecked(Int64.maxValue);  // None (overflow detected)
```

```
// Bit twiddling
(0b1010).countOnes      // 2
(1).shiftLeft(by: 4)    // 16
(-1).leadingZeros       // 0  (all bits set)
```

### Representation

A single `lang.i8` field. No padding, no headers — bit-identical
to the corresponding C type.

_Defined in `lang/std/num/int8.ks`._

### Members

#### initializer `Default`

```kestrel
public init()
```

Creates the zero value, satisfying `Defaultable`.

##### Examples

```
let n = Int64();   // 0
```

_Defined in `lang/std/num/int8.ks`._

#### initializer `From Integer`

```kestrel
public init(from: Int16)
```

Converts from `Int16`. Narrowing conversions truncate the high
bits; signed→unsigned reinterprets the bit pattern.

_Defined in `lang/std/num/int8.ks`._

#### initializer `From Integer`

```kestrel
public init(from: Int32)
```

Converts from `Int32`. Narrowing conversions truncate the high
bits; signed→unsigned reinterprets the bit pattern.

_Defined in `lang/std/num/int8.ks`._

#### initializer `From Integer`

```kestrel
public init(from: Int64)
```

Converts from `Int64`. Narrowing conversions truncate the high
bits; signed→unsigned reinterprets the bit pattern.

_Defined in `lang/std/num/int8.ks`._

#### initializer `From Integer`

```kestrel
public init(from: UInt8)
```

Converts from `UInt8`. Narrowing conversions truncate the high
bits; signed→unsigned reinterprets the bit pattern.

_Defined in `lang/std/num/int8.ks`._

#### initializer `From Integer`

```kestrel
public init(from: UInt16)
```

Converts from `UInt16`. Narrowing conversions truncate the high
bits; signed→unsigned reinterprets the bit pattern.

_Defined in `lang/std/num/int8.ks`._

#### initializer `From Integer`

```kestrel
public init(from: UInt32)
```

Converts from `UInt32`. Narrowing conversions truncate the high
bits; signed→unsigned reinterprets the bit pattern.

_Defined in `lang/std/num/int8.ks`._

#### initializer `From Integer`

```kestrel
public init(from: UInt64)
```

Converts from `UInt64`. Narrowing conversions truncate the high
bits; signed→unsigned reinterprets the bit pattern.

_Defined in `lang/std/num/int8.ks`._

#### initializer `From Raw`

```kestrel
init(raw: lang.i8)
```

Wraps an existing `lang.i8` without conversion. Internal
constructor used by intrinsics; not part of the public API.

_Defined in `lang/std/num/int8.ks`._

#### initializer `Int Literal`

```kestrel
public init(intLiteral: lang.i64)
```

Compiler-emitted bridge that turns an integer literal into a Int8.

You will rarely call this directly — write the literal and let the
`ExpressibleByIntLiteral` protocol pick it up. For widths smaller than
64 bits the literal is truncated with `lang.cast_i64_i8`.

##### Examples

```
let n: Int64 = 42;            // implicit
let m = Int64(intLiteral: 42);  // explicit
```

_Defined in `lang/std/num/int8.ks`._

#### function `absChecked`

```kestrel
public func absChecked() -> Int8?
```

Absolute value that returns `None` for `minValue` (whose absolute
value overflows).

_Defined in `lang/std/num/int8.ks`._

#### function `absSaturating`

```kestrel
public func absSaturating() -> Int8
```

Absolute value that returns `maxValue` instead of wrapping `minValue`.

_Defined in `lang/std/num/int8.ks`._

#### function `addChecked`

```kestrel
public func addChecked(Int8) -> Int8?
```

Wrapping addition that returns `None` instead of overflowing.

_Defined in `lang/std/num/int8.ks`._

#### function `addSaturating`

```kestrel
public func addSaturating(Int8) -> Int8
```

Addition that clamps to `maxValue`/`minValue` instead of wrapping.

_Defined in `lang/std/num/int8.ks`._

#### field `bitWidth`

```kestrel
public static var bitWidth: Int64 { get }
```

The width in bits (8). Useful for shift bounds and bit-walks.

_Defined in `lang/std/num/int8.ks`._

#### field `byteSwapped`

```kestrel
public var byteSwapped: Int8 { get }
```

Value with its byte order reversed. Use to convert between big- and
little-endian; lowered to a `bswap` intrinsic.

_Defined in `lang/std/num/int8.ks`._

#### function `clamp`

```kestrel
public func clamp(Int8, Int8) -> Int8
```

Clamps `self` into `[min, max]`. Caller is responsible for ensuring
`min <= max`; otherwise the result is undefined.

##### Examples

```
(5).clamp(min: 0, max: 10);    // 5
(-5).clamp(min: 0, max: 10);   // 0
(15).clamp(min: 0, max: 10);   // 10
```

_Defined in `lang/std/num/int8.ks`._

#### field `countOnes`

```kestrel
public var countOnes: Int64 { get }
```

Population count — the number of `1` bits in the binary representation.

Lowered to a `popcount` intrinsic where the target supports it.

##### Examples

```
(0b1010).countOnes;  // 2
(0b1111).countOnes;  // 4
(0).countOnes;       // 0
```

_Defined in `lang/std/num/int8.ks`._

#### field `countZeros`

```kestrel
public var countZeros: Int64 { get }
```

Complement of `countOnes`: equal to `bitWidth - countOnes`.

_Defined in `lang/std/num/int8.ks`._

#### function `divideChecked`

```kestrel
public func divideChecked(Int8) -> Int8?
```

Division that returns `None` for divide-by-zero or for the
`minValue / -1` overflow case.

_Defined in `lang/std/num/int8.ks`._

#### function `fromBytes`

```kestrel
public static func fromBytes(std.collections.Array[UInt8]) -> Int8?
```

Reassembles a `Int8` from 1 bytes in native (host) byte
order. Returns `None` if the input is not exactly 1 bytes long.

_Defined in `lang/std/num/int8.ks`._

#### function `fromBytesBigEndian`

```kestrel
public static func fromBytesBigEndian(std.collections.Array[UInt8]) -> Int8?
```

Reassembles a `Int8` from 1 bytes in big-endian order.
Returns `None` if the input is not exactly 1 bytes long.

_Defined in `lang/std/num/int8.ks`._

#### function `fromBytesLittleEndian`

```kestrel
public static func fromBytesLittleEndian(std.collections.Array[UInt8]) -> Int8?
```

Reassembles a `Int8` from 1 bytes in little-endian order.
Returns `None` if the input is not exactly 1 bytes long.

_Defined in `lang/std/num/int8.ks`._

#### function `gcd`

```kestrel
public func gcd(Int8) -> Int8
```

Greatest common divisor via Euclidean algorithm. For signed types
the inputs are taken absolute first; the result is always non-negative.

##### Examples

```
(12).gcd(8);   // 4
(17).gcd(5);   // 1   (coprime)
(-12).gcd(8);  // 4
```

_Defined in `lang/std/num/int8.ks`._

#### field `isNegative`

```kestrel
public var isNegative: Bool { get }
```

True when `self < 0`.

_Defined in `lang/std/num/int8.ks`._

#### field `isPositive`

```kestrel
public var isPositive: Bool { get }
```

True when `self > 0`.

_Defined in `lang/std/num/int8.ks`._

#### field `isPowerOfTwo`

```kestrel
public var isPowerOfTwo: Bool { get }
```

True when the value is a positive power of two (`2^k` for `k >= 0`).

Zero and negatives are excluded. Cheap branchless test built on
`x & (x - 1) == 0`.

##### Examples

```
(1).isPowerOfTwo;   // true  (2^0)
(4).isPowerOfTwo;   // true  (2^2)
(3).isPowerOfTwo;   // false
(0).isPowerOfTwo;   // false
```

_Defined in `lang/std/num/int8.ks`._

#### field `isZero`

```kestrel
public var isZero: Bool { get }
```

True when `self == 0`.

_Defined in `lang/std/num/int8.ks`._

#### function `lcm`

```kestrel
public func lcm(Int8) -> Int8
```

Least common multiple, computed as `|self| / gcd(self, other) * |other|`
to avoid intermediate overflow. Returns zero if either input is zero.

##### Examples

```
(4).lcm(6);   // 12
(3).lcm(5);   // 15
(0).lcm(7);   // 0
```

_Defined in `lang/std/num/int8.ks`._

#### field `leadingZeros`

```kestrel
public var leadingZeros: Int64 { get }
```

Number of leading zero bits, counting from the most-significant end.

For zero, returns `bitWidth`.

##### Examples

```
(1).leadingZeros;   // bitWidth - 1
(0).leadingZeros;   // bitWidth
```

_Defined in `lang/std/num/int8.ks`._

#### field `maxValue`

```kestrel
public static var maxValue: Int8 { get }
```

The largest representable value.
This is 2^7 - 1 (127).

_Defined in `lang/std/num/int8.ks`._

#### field `minValue`

```kestrel
public static var minValue: Int8 { get }
```

The smallest representable value.
This is -2^7 (-128).
Note that for signed types `minValue.negate()` overflows back to
itself; use `negateChecked()` if you need to detect that.

_Defined in `lang/std/num/int8.ks`._

#### function `multiplyChecked`

```kestrel
public func multiplyChecked(Int8) -> Int8?
```

Wrapping multiplication that returns `None` instead of overflowing.
Implemented by multiplying then dividing back; replace with an
overflow-detecting intrinsic when one is available.

_Defined in `lang/std/num/int8.ks`._

#### function `multiplySaturating`

```kestrel
public func multiplySaturating(Int8) -> Int8
```

Multiplication that clamps to `maxValue`/`minValue` instead of wrapping.
The clamp direction follows the algebraic sign of the would-be result.

_Defined in `lang/std/num/int8.ks`._

#### function `negateChecked`

```kestrel
public func negateChecked() -> Int8?
```

Negation that returns `None` for `minValue` (whose negation overflows).

_Defined in `lang/std/num/int8.ks`._

#### function `negateSaturating`

```kestrel
public func negateSaturating() -> Int8
```

Negation that returns `maxValue` instead of wrapping `minValue`.

_Defined in `lang/std/num/int8.ks`._

#### function `parse`

```kestrel
public static func parse(String) -> Int8?
```

Parses a base-10 integer literal, optionally prefixed with `+` or
`-`. Returns `None` for an empty string, a non-digit character,
or a value that does not fit in `Int8`.

##### Examples

```
Int8.parse(string: "42");    // Some(42)
Int8.parse(string: "-7");    // Some(-7)
Int8.parse(string: "abc");   // None
Int8.parse(string: "");      // None
```

_Defined in `lang/std/num/int8.ks`._

#### function `parse`

```kestrel
public static func parse(String, Int64) -> Int8?
```

Parses an integer in `radix` (base 2–36 inclusive). Letters a–z are
case-insensitive and represent digit values 10–35. Returns `None`
for an out-of-range radix, an empty string, an unrecognised digit,
or a value that overflows `Int8`.

##### Examples

```
Int8.parse(string: "ff", radix: 16);     // Some(255 if it fits, else None)
Int8.parse(string: "101010", radix: 2);  // Some(42)
Int8.parse(string: "z", radix: 36);      // Some(35)
```

_Defined in `lang/std/num/int8.ks`._

#### function `pow`

```kestrel
public func pow(Int64) -> Int8
```

Raises `self` to `exponent` via binary exponentiation. Wraps on
overflow. Negative exponents return zero (integer truncation of
the would-be fraction).

##### Examples

```
(2).pow(10);  // 1024
(3).pow(4);   // 81
(5).pow(-1);  // 0
```

_Defined in `lang/std/num/int8.ks`._

#### field `raw`

```kestrel
public var raw: lang.i8
```

The underlying primitive `lang.i8` value. Exposed for FFI
and intrinsic use; prefer the typed surface for everything else.

_Defined in `lang/std/num/int8.ks`._

#### function `rotateLeft`

```kestrel
public func rotateLeft(by: Int64) -> Int8
```

Rotates bits left by `count`, modulo `bitWidth`. Bits shifted past the
MSB re-enter at the LSB.

_Defined in `lang/std/num/int8.ks`._

#### function `rotateRight`

```kestrel
public func rotateRight(by: Int64) -> Int8
```

Rotates bits right by `count`, modulo `bitWidth`. Mirror of
`rotateLeft`.

_Defined in `lang/std/num/int8.ks`._

#### field `sign`

```kestrel
public var sign: Int8 { get }
```

Sign as a `Int8`: `-1`, `0`, or `1`.

_Defined in `lang/std/num/int8.ks`._

#### function `subtractChecked`

```kestrel
public func subtractChecked(Int8) -> Int8?
```

Wrapping subtraction that returns `None` instead of overflowing.

_Defined in `lang/std/num/int8.ks`._

#### function `subtractSaturating`

```kestrel
public func subtractSaturating(Int8) -> Int8
```

Subtraction that clamps to `maxValue`/`minValue` instead of wrapping.

_Defined in `lang/std/num/int8.ks`._

#### function `toBytes`

```kestrel
public func toBytes() -> std.collections.Array[UInt8]
```

Splits this integer into 1 bytes in *native* (host) byte order.
Use `toBytesBigEndian` / `toBytesLittleEndian` when serialising for
a fixed wire format.

##### Examples

```
let bytes = Int8.maxValue.toBytes();   // 1 bytes, host order
```

_Defined in `lang/std/num/int8.ks`._

#### function `toBytesBigEndian`

```kestrel
public func toBytesBigEndian() -> std.collections.Array[UInt8]
```

Splits this integer into 1 bytes in big-endian order (most
significant byte first — i.e. network byte order).

_Defined in `lang/std/num/int8.ks`._

#### function `toBytesLittleEndian`

```kestrel
public func toBytesLittleEndian() -> std.collections.Array[UInt8]
```

Splits this integer into 1 bytes in little-endian order (least
significant byte first).

_Defined in `lang/std/num/int8.ks`._

#### field `trailingZeros`

```kestrel
public var trailingZeros: Int64 { get }
```

Number of trailing zero bits. Equal to `log2(self & -self)` for non-zero
values; returns `bitWidth` for zero. Useful for finding the largest
power of two dividing the value.

_Defined in `lang/std/num/int8.ks`._

### Implements `SignedInteger`

#### function `abs`

```kestrel
public func abs() -> Int8
```

Absolute value. Wraps at the minimum value
(`Int8.minValue.abs() == Int8.minValue`); use
`absChecked` if that's a problem.

_Defined in `lang/std/num/int8.ks`._

### Implements `Steppable`

#### function `predecessor`

```kestrel
public func predecessor() -> Int8
```

Predecessor — `self - 1`. Wraps at `minValue`.

_Defined in `lang/std/num/int8.ks`._

#### function `successor`

```kestrel
public func successor() -> Int8
```

Successor — `self + 1`. Wraps at `maxValue`. Used by `for-in` over
integer ranges.

_Defined in `lang/std/num/int8.ks`._

### Implements `Comparable`

#### function `compare`

```kestrel
public func compare(Int8) -> Ordering
```

Three-way comparison returning an `Ordering`. Signed types compare
using two's-complement ordering; unsigned types use natural ordering.

##### Examples

```
(1).compare(other: 2);   // .Less
(2).compare(other: 2);   // .Equal
(3).compare(other: 2);   // .Greater
```

_Defined in `lang/std/num/int8.ks`._

### Implements `Equatable`

#### function `equals`

```kestrel
public func equals(Int8) -> Bool
```

Bit-for-bit equality. Backs the `==` operator.

##### Examples

```
(42).equals(other: 42);  // true
42 == 42;                // true
```

_Defined in `lang/std/num/int8.ks`._

### Implements `Matchable`

#### function `matches`

```kestrel
public func matches(Int8) -> Bool
```

Pattern-matching hook for `Matchable`. Identical to `equals`.

_Defined in `lang/std/num/int8.ks`._

### Implements `Formattable`

#### function `format`

```kestrel
public func format(FormatOptions) -> String
```

Renders the integer to a `String`, honouring the supplied
`FormatOptions`. Implements the `Formattable` protocol.

Recognised options:
- `radix` — base in `[2, 36]`; out-of-range values fall back to 10.
- `width` — minimum output width; shorter values are padded.
- `fill` / `alignment` — padding character and side.
- `sign` — `.Negative` (default), `.Always`, or `.Space`.
- `uppercase` — uppercase hex digits.
- `alternate` — emit the `0b` / `0o` / `0x` prefix.

##### Examples

```
(42).format();                                           // "42"
(255).format(options: .{radix: 16});                     // "ff"
(255).format(options: .{radix: 16, uppercase: true});    // "FF"
(255).format(options: .{radix: 16, alternate: true});    // "0xff"
(42).format(options: .{radix: 2, alternate: true});      // "0b101010"
(42).format(options: .{width: .Some(5), fill: '0'});     // "00042"
(-42).format(options: .{sign: .Always});                 // "-42"
```

_Defined in `lang/std/num/int8.ks`._

### Implements `Hash`

#### function `hash`

```kestrel
public func hash[H](into: mutating H) where H: Hasher
```

Feeds the raw bytes of this value into `hasher`. Endianness-agnostic
only within a single process — do not persist hashes across builds.

_Defined in `lang/std/num/int8.ks`._

### Implements `Addable`

#### typealias `Output`

```kestrel
type Output = Int8
```

_Defined in `lang/std/num/int8.ks`._

#### typealias `Output`

```kestrel
type Output = Int8
```

_Defined in `lang/std/num/int8.ks`._

#### typealias `Output`

```kestrel
type Output = Int8
```

_Defined in `lang/std/num/int8.ks`._

#### typealias `Output`

```kestrel
type Output = Int8
```

_Defined in `lang/std/num/int8.ks`._

#### typealias `Output`

```kestrel
type Output = Int8
```

_Defined in `lang/std/num/int8.ks`._

#### typealias `Output`

```kestrel
type Output = Int8
```

_Defined in `lang/std/num/int8.ks`._

#### typealias `Output`

```kestrel
type Output = Int8
```

_Defined in `lang/std/num/int8.ks`._

#### typealias `Output`

```kestrel
type Output = Int8
```

_Defined in `lang/std/num/int8.ks`._

#### typealias `Output`

```kestrel
type Output = Int8
```

_Defined in `lang/std/num/int8.ks`._

#### typealias `Output`

```kestrel
type Output = Int8
```

_Defined in `lang/std/num/int8.ks`._

#### typealias `Output`

```kestrel
type Output = Int8
```

_Defined in `lang/std/num/int8.ks`._

#### typealias `Output`

```kestrel
type Output = Int8
```

_Defined in `lang/std/num/int8.ks`._

#### typealias `Output`

```kestrel
type Output = Range[Int8]
```

_Defined in `lang/std/num/int8.ks`._

#### typealias `Output`

```kestrel
type Output = ClosedRange[Int8]
```

_Defined in `lang/std/num/int8.ks`._

#### function `add`

```kestrel
public func add(Int8) -> Int8
```

`self + other`, wrapping on overflow. Use `addChecked` to detect or
`addSaturating` to clamp.

_Defined in `lang/std/num/int8.ks`._

#### field `zero`

```kestrel
public static var zero: Int8 { get }
```

The additive identity, `0`.

_Defined in `lang/std/num/int8.ks`._

### Implements `Subtractable`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/arithmetic.ks`._

#### function `subtract`

```kestrel
public func subtract(Int8) -> Int8
```

`self - other`, wrapping on overflow.

_Defined in `lang/std/num/int8.ks`._

### Implements `Multipliable`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/arithmetic.ks`._

#### function `multiply`

```kestrel
public func multiply(Int8) -> Int8
```

`self * other`, wrapping on overflow.

_Defined in `lang/std/num/int8.ks`._

#### field `one`

```kestrel
public static var one: Int8 { get }
```

The multiplicative identity, `1`.

_Defined in `lang/std/num/int8.ks`._

### Implements `Divisible`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/arithmetic.ks`._

#### function `divide`

```kestrel
public func divide(Int8) -> Int8
```

Truncating integer division (`self / other`). For signed types,
`minValue / -1` wraps; use `divideChecked` to detect.

##### Errors

Traps on division by zero (LLVM `udiv`/`sdiv` are UB on zero — the
process aborts before producing a result).

_Defined in `lang/std/num/int8.ks`._

### Implements `Modulo`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/arithmetic.ks`._

#### function `modulo`

```kestrel
public func modulo(Int8) -> Int8
```

`self % other` — truncated remainder; the result has the sign of
`self` for signed types.

##### Errors

Traps on division by zero, like `divide`.

_Defined in `lang/std/num/int8.ks`._

### Implements `Negatable`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/arithmetic.ks`._

#### function `negate`

```kestrel
public func negate() -> Int8
```

Two's-complement negation. Wraps at the minimum value:
`Int8.minValue.negate() == Int8.minValue`. Use
`negateChecked` to surface the overflow.

_Defined in `lang/std/num/int8.ks`._

### Implements `BitwiseAnd`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/bitwise.ks`._

#### function `bitwiseAnd`

```kestrel
public func bitwiseAnd(Int8) -> Int8
```

Bitwise AND. `0b1010 & 0b1100 == 0b1000`.

_Defined in `lang/std/num/int8.ks`._

### Implements `BitwiseOr`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/bitwise.ks`._

#### function `bitwiseOr`

```kestrel
public func bitwiseOr(Int8) -> Int8
```

Bitwise OR. `0b1010 | 0b1100 == 0b1110`.

_Defined in `lang/std/num/int8.ks`._

### Implements `BitwiseXor`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/bitwise.ks`._

#### function `bitwiseXor`

```kestrel
public func bitwiseXor(Int8) -> Int8
```

Bitwise XOR. `0b1010 ^ 0b1100 == 0b0110`.

_Defined in `lang/std/num/int8.ks`._

### Implements `BitwiseNot`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/bitwise.ks`._

#### function `bitwiseNot`

```kestrel
public func bitwiseNot() -> Int8
```

Bitwise NOT — flips all bits. For signed types this is `-self - 1`.

_Defined in `lang/std/num/int8.ks`._

### Implements `LeftShift`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/bitwise.ks`._

#### function `shiftLeft`

```kestrel
public func shiftLeft(by: lang.i64) -> Int8
```

Left shift by `count`. Behavior is undefined when `count >= bitWidth`
— pre-mask the count if you can't guarantee the bound.

_Defined in `lang/std/num/int8.ks`._

### Implements `RightShift`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/bitwise.ks`._

#### function `shiftRight`

```kestrel
public func shiftRight(by: lang.i64) -> Int8
```

Right shift by `count`. Arithmetic (sign-extending) for signed types,
logical (zero-filling) for unsigned. Same `count` precondition as
`shiftLeft`.

_Defined in `lang/std/num/int8.ks`._

### Implements `AddAssign`

#### function `addAssign`

```kestrel
public mutating func addAssign(Int8)
```

`self += other`

_Defined in `lang/std/num/int8.ks`._

### Implements `SubtractAssign`

#### function `subtractAssign`

```kestrel
public mutating func subtractAssign(Int8)
```

`self -= other`

_Defined in `lang/std/num/int8.ks`._

### Implements `MultiplyAssign`

#### function `multiplyAssign`

```kestrel
public mutating func multiplyAssign(Int8)
```

`self *= other`

_Defined in `lang/std/num/int8.ks`._

### Implements `DivideAssign`

#### function `divideAssign`

```kestrel
public mutating func divideAssign(Int8)
```

`self /= other`

_Defined in `lang/std/num/int8.ks`._

### Implements `ModuloAssign`

#### function `modAssign`

```kestrel
public mutating func modAssign(Int8)
```

`self %= other`

_Defined in `lang/std/num/int8.ks`._

### Implements `BitwiseAndAssign`

#### function `bitwiseAndAssign`

```kestrel
public mutating func bitwiseAndAssign(Int8)
```

`self &= other`

_Defined in `lang/std/num/int8.ks`._

### Implements `BitwiseOrAssign`

#### function `bitwiseOrAssign`

```kestrel
public mutating func bitwiseOrAssign(Int8)
```

`self |= other`

_Defined in `lang/std/num/int8.ks`._

### Implements `BitwiseXorAssign`

#### function `bitwiseXorAssign`

```kestrel
public mutating func bitwiseXorAssign(Int8)
```

`self ^= other`

_Defined in `lang/std/num/int8.ks`._

### Implements `LeftShiftAssign`

#### function `shiftLeftAssign`

```kestrel
public mutating func shiftLeftAssign(by: lang.i64)
```

`self <<= count`

_Defined in `lang/std/num/int8.ks`._

### Implements `RightShiftAssign`

#### function `shiftRightAssign`

```kestrel
public mutating func shiftRightAssign(by: lang.i64)
```

`self >>= count`

_Defined in `lang/std/num/int8.ks`._

### Implements `ExpressibleByIntLiteral`

#### initializer `Int Literal`

```kestrel
init(intLiteral: lang.i64)
```

Builds an instance from an integer literal.

_Defined in `lang/std/core/literals.ks`._

### Implements `Defaultable`

#### initializer `Default`

```kestrel
init()
```

Builds the default-valued instance.

_Defined in `lang/std/core/protocols.ks`._

### Implements `RangeConstructible`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/range.ks`._

#### function `exclusiveRange`

```kestrel
public func exclusiveRange(to: Int8) -> Range[Int8]
```

Builds a half-open range `self..<end`. Sugar for the `..<` operator.

_Defined in `lang/std/num/int8.ks`._

### Implements `ClosedRangeConstructible`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/range.ks`._

#### function `inclusiveRange`

```kestrel
public func inclusiveRange(to: Int8) -> ClosedRange[Int8]
```

Builds a closed range `self..=end`. Sugar for the `..=` operator.

_Defined in `lang/std/num/int8.ks`._

### Implements `Convertible`

#### initializer `From Source`

```kestrel
init(from: From)
```

Creates an instance from `value`.

_Defined in `lang/std/core/convertible.ks`._

## struct `Lcg64`

```kestrel
public struct Lcg64 { /* private fields */ }
```

A 64-bit linear congruential generator. Cheap, allocation-free, and
adequate for shuffling, fuzz seeds, and simulation noise — *not* for
cryptographic use, key generation, or anything an adversary observes.

Constants come from Numerical Recipes and give a full period of `2^64`:

- multiplier `a = 6364136223846793005`
- increment  `c = 1442695040888963407`

The state update is `state = state * a + c`, returning the new state.

### Examples

```
var rng = Lcg64(seed: 12345);
let v1 = rng.nextUInt64();
let v2 = rng.nextUInt64();   // distinct from v1
```

### Representation

One `UInt64` field — the mutable generator state.

_Defined in `lang/std/num/random.ks`._

### Members

#### initializer `Default`

```kestrel
public init()
```

Creates a generator with a hard-coded default seed
(`88172645463325252`). Always produces the same stream — provide an
explicit seed via `init(seed:)` when you need variation between runs.

_Defined in `lang/std/num/random.ks`._

#### initializer `Seeded`

```kestrel
public init(seed: UInt64)
```

Creates a generator initialised with `seed`. Different seeds produce
independent streams; the same seed always reproduces the same stream
(useful for deterministic tests).

##### Examples

```
var rng = Lcg64(seed: 42);
```

_Defined in `lang/std/num/random.ks`._

### Implements `RandomNumberGenerator`

#### function `nextUInt64`

```kestrel
public mutating func nextUInt64() -> UInt64
```

Advances the state once and returns the new value. `O(1)` and
allocation-free.

_Defined in `lang/std/num/random.ks`._

### Implements `Defaultable`

#### initializer `Default`

```kestrel
init()
```

Builds the default-valued instance.

_Defined in `lang/std/core/protocols.ks`._

## protocol `RandomNumberGenerator`

```kestrel
public protocol RandomNumberGenerator
```

A source of pseudo-random `UInt64` values. Implementers expose a single
raw-uniform primitive; the extension on this protocol layers ergonomic
helpers on top.

Conformers are free to choose any algorithm they like — the protocol
makes no statement about cryptographic strength, period, or bias. Pick
`Lcg64` for cheap non-cryptographic randomness; bring your own type for
anything stronger.

### Examples

```
struct MyRng: RandomNumberGenerator {
    var state: UInt64;

    mutating func nextUInt64() -> UInt64 {
        // mix state, return a fresh value
    }
}
```

_Defined in `lang/std/num/random.ks`._

### Members

#### function `nextInt`

```kestrel
public mutating func nextInt(below: Int64) -> Int64
```

Returns a uniformly distributed integer in `[0, upperBound)`.
Returns `0` when `upperBound <= 0` rather than panicking.

Uses naive modulo for simplicity — for `upperBound` close to
`UInt64.maxValue` the result has slight bias toward smaller values.
If you need exact uniformity, sample `nextUInt64()` and reject.

##### Examples

```
var rng = Lcg64(seed: 42);
let roll = rng.nextInt(below: 6);   // 0..5
```

_Defined in `lang/std/num/random.ks`._

#### function `nextUInt64`

```kestrel
mutating func nextUInt64() -> UInt64
```

Returns the next `UInt64` from the stream and advances internal
state. Each call should be independent and uniformly distributed
over the full `UInt64` range — implementers that can't promise
uniformity (e.g. very small periods) should document the bias.

_Defined in `lang/std/num/random.ks`._

## protocol `SignedInteger`

```kestrel
public protocol SignedInteger
```

Marker protocol for signed integer types. The `abs()` requirement is
what justifies treating these uniformly in generic code — unsigned
integers can't satisfy it without changing semantics.

_Defined in `lang/std/num/numeric.ks`._

### Members

#### function `abs`

```kestrel
func abs() -> Self
```

Absolute value. For two's-complement types this can overflow at
`minValue`; consumers that need a total function should use
`absChecked()` from the concrete type instead.

_Defined in `lang/std/num/numeric.ks`._

## protocol `Steppable`

```kestrel
public protocol Steppable
```

A type whose values can be stepped one position at a time. Underpins
`for-in` over integer ranges and any other "next/previous" walk where
the step size is implicit (`1` for integers).

`successor` and `predecessor` should be inverses for every interior
value; behaviour at the type's edges (`Int64.maxValue.successor()`,
for example) follows the same wrapping rules as `add`/`subtract`.

_Defined in `lang/std/num/numeric.ks`._

### Members

#### function `predecessor`

```kestrel
func predecessor() -> Self
```

The previous value in the sequence. For integers this is `self - 1`.

_Defined in `lang/std/num/numeric.ks`._

#### function `successor`

```kestrel
func successor() -> Self
```

The next value in the sequence. For integers this is `self + 1`.

_Defined in `lang/std/num/numeric.ks`._

## typealias `UInt`

```kestrel
public type UInt = UInt64
```

Platform-sized unsigned integer — currently always `UInt64`.

_Defined in `lang/std/num/uint64.ks`._

## struct `UInt16`

```kestrel
public struct UInt16 { /* private fields */ }
```

A 16-bit unsigned integer.

UInt16 is the 16-bit member of the integer family. The same surface
area is provided across all widths; switch widths to trade range for memory
or to match an FFI ABI. Arithmetic wraps on overflow by default — use the
`*Checked` variants for overflow detection or `*Saturating` to clamp to
`minValue`/`maxValue`. The type is `FFISafe` and lays out as a single
`lang.i16` so it can cross C boundaries unchanged.

### Examples

```
let a: Int64 = 100;
let b = a + 50;        // 150
let c = a * 2;         // 200
let d = a.addChecked(Int64.maxValue);  // None (overflow detected)
```

```
// Bit twiddling
(0b1010).countOnes      // 2
(1).shiftLeft(by: 4)    // 16
(-1).leadingZeros       // 0  (all bits set)
```

### Representation

A single `lang.i16` field. No padding, no headers — bit-identical
to the corresponding C type.

_Defined in `lang/std/num/uint16.ks`._

### Members

#### initializer `Default`

```kestrel
public init()
```

Creates the zero value, satisfying `Defaultable`.

##### Examples

```
let n = Int64();   // 0
```

_Defined in `lang/std/num/uint16.ks`._

#### initializer `From Integer`

```kestrel
public init(from: Int8)
```

Converts from `Int8`. Narrowing conversions truncate the high
bits; signed→unsigned reinterprets the bit pattern.

_Defined in `lang/std/num/uint16.ks`._

#### initializer `From Integer`

```kestrel
public init(from: Int16)
```

Converts from `Int16`. Narrowing conversions truncate the high
bits; signed→unsigned reinterprets the bit pattern.

_Defined in `lang/std/num/uint16.ks`._

#### initializer `From Integer`

```kestrel
public init(from: Int32)
```

Converts from `Int32`. Narrowing conversions truncate the high
bits; signed→unsigned reinterprets the bit pattern.

_Defined in `lang/std/num/uint16.ks`._

#### initializer `From Integer`

```kestrel
public init(from: Int64)
```

Converts from `Int64`. Narrowing conversions truncate the high
bits; signed→unsigned reinterprets the bit pattern.

_Defined in `lang/std/num/uint16.ks`._

#### initializer `From Integer`

```kestrel
public init(from: UInt8)
```

Converts from `UInt8`. Narrowing conversions truncate the high
bits; signed→unsigned reinterprets the bit pattern.

_Defined in `lang/std/num/uint16.ks`._

#### initializer `From Integer`

```kestrel
public init(from: UInt32)
```

Converts from `UInt32`. Narrowing conversions truncate the high
bits; signed→unsigned reinterprets the bit pattern.

_Defined in `lang/std/num/uint16.ks`._

#### initializer `From Integer`

```kestrel
public init(from: UInt64)
```

Converts from `UInt64`. Narrowing conversions truncate the high
bits; signed→unsigned reinterprets the bit pattern.

_Defined in `lang/std/num/uint16.ks`._

#### initializer `From Raw`

```kestrel
init(raw: lang.i16)
```

Wraps an existing `lang.i16` without conversion. Internal
constructor used by intrinsics; not part of the public API.

_Defined in `lang/std/num/uint16.ks`._

#### initializer `Int Literal`

```kestrel
public init(intLiteral: lang.i64)
```

Compiler-emitted bridge that turns an integer literal into a UInt16.

You will rarely call this directly — write the literal and let the
`ExpressibleByIntLiteral` protocol pick it up. For widths smaller than
64 bits the literal is truncated with `lang.cast_i64_i16`.

##### Examples

```
let n: Int64 = 42;            // implicit
let m = Int64(intLiteral: 42);  // explicit
```

_Defined in `lang/std/num/uint16.ks`._

#### function `addChecked`

```kestrel
public func addChecked(UInt16) -> UInt16?
```

Wrapping addition that returns `None` on overflow. For unsigned types
overflow is detected via `result < self`.

_Defined in `lang/std/num/uint16.ks`._

#### function `addSaturating`

```kestrel
public func addSaturating(UInt16) -> UInt16
```

Addition that clamps to `maxValue` on overflow.

_Defined in `lang/std/num/uint16.ks`._

#### field `bitWidth`

```kestrel
public static var bitWidth: Int64 { get }
```

The width in bits (16). Useful for shift bounds and bit-walks.

_Defined in `lang/std/num/uint16.ks`._

#### field `byteSwapped`

```kestrel
public var byteSwapped: UInt16 { get }
```

Value with its byte order reversed. Use to convert between big- and
little-endian; lowered to a `bswap` intrinsic.

_Defined in `lang/std/num/uint16.ks`._

#### function `clamp`

```kestrel
public func clamp(UInt16, UInt16) -> UInt16
```

Clamps `self` into `[min, max]`. Caller is responsible for ensuring
`min <= max`; otherwise the result is undefined.

##### Examples

```
(5).clamp(min: 0, max: 10);    // 5
(-5).clamp(min: 0, max: 10);   // 0
(15).clamp(min: 0, max: 10);   // 10
```

_Defined in `lang/std/num/uint16.ks`._

#### field `countOnes`

```kestrel
public var countOnes: Int64 { get }
```

Population count — the number of `1` bits in the binary representation.

Lowered to a `popcount` intrinsic where the target supports it.

##### Examples

```
(0b1010).countOnes;  // 2
(0b1111).countOnes;  // 4
(0).countOnes;       // 0
```

_Defined in `lang/std/num/uint16.ks`._

#### field `countZeros`

```kestrel
public var countZeros: Int64 { get }
```

Complement of `countOnes`: equal to `bitWidth - countOnes`.

_Defined in `lang/std/num/uint16.ks`._

#### function `divideChecked`

```kestrel
public func divideChecked(UInt16) -> UInt16?
```

Division that returns `None` for divide-by-zero.

_Defined in `lang/std/num/uint16.ks`._

#### function `fromBytes`

```kestrel
public static func fromBytes(std.collections.Array[UInt8]) -> UInt16?
```

Reassembles a `UInt16` from 2 bytes in native (host) byte
order. Returns `None` if the input is not exactly 2 bytes long.

_Defined in `lang/std/num/uint16.ks`._

#### function `fromBytesBigEndian`

```kestrel
public static func fromBytesBigEndian(std.collections.Array[UInt8]) -> UInt16?
```

Reassembles a `UInt16` from 2 bytes in big-endian order.
Returns `None` if the input is not exactly 2 bytes long.

_Defined in `lang/std/num/uint16.ks`._

#### function `fromBytesLittleEndian`

```kestrel
public static func fromBytesLittleEndian(std.collections.Array[UInt8]) -> UInt16?
```

Reassembles a `UInt16` from 2 bytes in little-endian order.
Returns `None` if the input is not exactly 2 bytes long.

_Defined in `lang/std/num/uint16.ks`._

#### function `gcd`

```kestrel
public func gcd(UInt16) -> UInt16
```

Greatest common divisor via Euclidean algorithm. For signed types
the inputs are taken absolute first; the result is always non-negative.

##### Examples

```
(12).gcd(8);   // 4
(17).gcd(5);   // 1   (coprime)
(-12).gcd(8);  // 4
```

_Defined in `lang/std/num/uint16.ks`._

#### field `isNegative`

```kestrel
public var isNegative: Bool { get }
```

Always `false` — unsigned types cannot be negative.

_Defined in `lang/std/num/uint16.ks`._

#### field `isPositive`

```kestrel
public var isPositive: Bool { get }
```

True when `self > 0`.

_Defined in `lang/std/num/uint16.ks`._

#### field `isPowerOfTwo`

```kestrel
public var isPowerOfTwo: Bool { get }
```

True when the value is a positive power of two (`2^k` for `k >= 0`).

Zero and negatives are excluded. Cheap branchless test built on
`x & (x - 1) == 0`.

##### Examples

```
(1).isPowerOfTwo;   // true  (2^0)
(4).isPowerOfTwo;   // true  (2^2)
(3).isPowerOfTwo;   // false
(0).isPowerOfTwo;   // false
```

_Defined in `lang/std/num/uint16.ks`._

#### field `isZero`

```kestrel
public var isZero: Bool { get }
```

True when `self == 0`.

_Defined in `lang/std/num/uint16.ks`._

#### function `lcm`

```kestrel
public func lcm(UInt16) -> UInt16
```

Least common multiple, computed as `|self| / gcd(self, other) * |other|`
to avoid intermediate overflow. Returns zero if either input is zero.

##### Examples

```
(4).lcm(6);   // 12
(3).lcm(5);   // 15
(0).lcm(7);   // 0
```

_Defined in `lang/std/num/uint16.ks`._

#### field `leadingZeros`

```kestrel
public var leadingZeros: Int64 { get }
```

Number of leading zero bits, counting from the most-significant end.

For zero, returns `bitWidth`.

##### Examples

```
(1).leadingZeros;   // bitWidth - 1
(0).leadingZeros;   // bitWidth
```

_Defined in `lang/std/num/uint16.ks`._

#### field `maxValue`

```kestrel
public static var maxValue: UInt16 { get }
```

The largest representable value.
This is 2^16 - 1 (65_535).

_Defined in `lang/std/num/uint16.ks`._

#### field `minValue`

```kestrel
public static var minValue: UInt16 { get }
```

The smallest representable value.
This is always 0 for unsigned types.
Note that for signed types `minValue.negate()` overflows back to
itself; use `negateChecked()` if you need to detect that.

_Defined in `lang/std/num/uint16.ks`._

#### function `multiplyChecked`

```kestrel
public func multiplyChecked(UInt16) -> UInt16?
```

Wrapping multiplication that returns `None` on overflow. Implemented
by multiplying then dividing back.

_Defined in `lang/std/num/uint16.ks`._

#### function `multiplySaturating`

```kestrel
public func multiplySaturating(UInt16) -> UInt16
```

Multiplication that clamps to `maxValue` on overflow.

_Defined in `lang/std/num/uint16.ks`._

#### function `parse`

```kestrel
public static func parse(String) -> UInt16?
```

Parses a base-10 unsigned integer literal, optionally prefixed
with `+`. A leading `-` is rejected. Returns `None` for an empty
string, a non-digit character, or a value that does not fit in
`UInt16`.

##### Examples

```
UInt16.parse(string: "42");   // Some(42)
UInt16.parse(string: "-1");   // None  (no sign for unsigned)
UInt16.parse(string: "");     // None
```

_Defined in `lang/std/num/uint16.ks`._

#### function `parse`

```kestrel
public static func parse(String, Int64) -> UInt16?
```

Parses an unsigned integer in `radix` (base 2–36 inclusive). Letters
a–z are case-insensitive and represent digit values 10–35. A
leading `+` is allowed but a leading `-` is rejected. Returns
`None` for an out-of-range radix, an empty string, an
unrecognised digit, or a value that overflows `UInt16`.

##### Examples

```
UInt16.parse(string: "ff", radix: 16);     // Some(255 if it fits, else None)
UInt16.parse(string: "101010", radix: 2);  // Some(42)
```

_Defined in `lang/std/num/uint16.ks`._

#### function `pow`

```kestrel
public func pow(Int64) -> UInt16
```

Raises `self` to `exponent` via binary exponentiation. Wraps on
overflow. Negative exponents return zero (integer truncation of
the would-be fraction).

##### Examples

```
(2).pow(10);  // 1024
(3).pow(4);   // 81
(5).pow(-1);  // 0
```

_Defined in `lang/std/num/uint16.ks`._

#### field `raw`

```kestrel
public var raw: lang.i16
```

The underlying primitive `lang.i16` value. Exposed for FFI
and intrinsic use; prefer the typed surface for everything else.

_Defined in `lang/std/num/uint16.ks`._

#### function `rotateLeft`

```kestrel
public func rotateLeft(by: Int64) -> UInt16
```

Rotates bits left by `count`, modulo `bitWidth`. Bits shifted past the
MSB re-enter at the LSB.

_Defined in `lang/std/num/uint16.ks`._

#### function `rotateRight`

```kestrel
public func rotateRight(by: Int64) -> UInt16
```

Rotates bits right by `count`, modulo `bitWidth`. Mirror of
`rotateLeft`.

_Defined in `lang/std/num/uint16.ks`._

#### field `sign`

```kestrel
public var sign: UInt16 { get }
```

Sign as a `UInt16`: `0` for zero, `1` otherwise (unsigned types
have no negative values).

_Defined in `lang/std/num/uint16.ks`._

#### function `subtractChecked`

```kestrel
public func subtractChecked(UInt16) -> UInt16?
```

Subtraction that returns `None` on underflow (`other > self`).

_Defined in `lang/std/num/uint16.ks`._

#### function `subtractSaturating`

```kestrel
public func subtractSaturating(UInt16) -> UInt16
```

Subtraction that clamps to `0` on underflow (unsigned types cannot
represent negative results).

_Defined in `lang/std/num/uint16.ks`._

#### function `toBytes`

```kestrel
public func toBytes() -> std.collections.Array[UInt8]
```

Splits this integer into 2 bytes in *native* (host) byte order.
Use `toBytesBigEndian` / `toBytesLittleEndian` when serialising for
a fixed wire format.

##### Examples

```
let bytes = UInt16.maxValue.toBytes();   // 2 bytes, host order
```

_Defined in `lang/std/num/uint16.ks`._

#### function `toBytesBigEndian`

```kestrel
public func toBytesBigEndian() -> std.collections.Array[UInt8]
```

Splits this integer into 2 bytes in big-endian order (most
significant byte first — i.e. network byte order).

_Defined in `lang/std/num/uint16.ks`._

#### function `toBytesLittleEndian`

```kestrel
public func toBytesLittleEndian() -> std.collections.Array[UInt8]
```

Splits this integer into 2 bytes in little-endian order (least
significant byte first).

_Defined in `lang/std/num/uint16.ks`._

#### field `trailingZeros`

```kestrel
public var trailingZeros: Int64 { get }
```

Number of trailing zero bits. Equal to `log2(self & -self)` for non-zero
values; returns `bitWidth` for zero. Useful for finding the largest
power of two dividing the value.

_Defined in `lang/std/num/uint16.ks`._

### Implements `Steppable`

#### function `predecessor`

```kestrel
public func predecessor() -> UInt16
```

Predecessor — `self - 1`. Wraps at `minValue`.

_Defined in `lang/std/num/uint16.ks`._

#### function `successor`

```kestrel
public func successor() -> UInt16
```

Successor — `self + 1`. Wraps at `maxValue`. Used by `for-in` over
integer ranges.

_Defined in `lang/std/num/uint16.ks`._

### Implements `Comparable`

#### function `compare`

```kestrel
public func compare(UInt16) -> Ordering
```

Three-way comparison returning an `Ordering`. Signed types compare
using two's-complement ordering; unsigned types use natural ordering.

##### Examples

```
(1).compare(other: 2);   // .Less
(2).compare(other: 2);   // .Equal
(3).compare(other: 2);   // .Greater
```

_Defined in `lang/std/num/uint16.ks`._

### Implements `Equatable`

#### function `equals`

```kestrel
public func equals(UInt16) -> Bool
```

Bit-for-bit equality. Backs the `==` operator.

##### Examples

```
(42).equals(other: 42);  // true
42 == 42;                // true
```

_Defined in `lang/std/num/uint16.ks`._

### Implements `Matchable`

#### function `matches`

```kestrel
public func matches(UInt16) -> Bool
```

Pattern-matching hook for `Matchable`. Identical to `equals`.

_Defined in `lang/std/num/uint16.ks`._

### Implements `Formattable`

#### function `format`

```kestrel
public func format(FormatOptions) -> String
```

Renders the integer to a `String`, honouring the supplied
`FormatOptions`. Implements the `Formattable` protocol.

Recognised options:
- `radix` — base in `[2, 36]`; out-of-range values fall back to 10.
- `width` — minimum output width; shorter values are padded.
- `fill` / `alignment` — padding character and side.
- `sign` — `.Negative` (default), `.Always`, or `.Space`.
- `uppercase` — uppercase hex digits.
- `alternate` — emit the `0b` / `0o` / `0x` prefix.

##### Examples

```
(42).format();                                           // "42"
(255).format(options: .{radix: 16});                     // "ff"
(255).format(options: .{radix: 16, uppercase: true});    // "FF"
(255).format(options: .{radix: 16, alternate: true});    // "0xff"
(42).format(options: .{radix: 2, alternate: true});      // "0b101010"
(42).format(options: .{width: .Some(5), fill: '0'});     // "00042"
(-42).format(options: .{sign: .Always});                 // "-42"
```

_Defined in `lang/std/num/uint16.ks`._

### Implements `Hash`

#### function `hash`

```kestrel
public func hash[H](into: mutating H) where H: Hasher
```

Feeds the raw bytes of this value into `hasher`. Endianness-agnostic
only within a single process — do not persist hashes across builds.

_Defined in `lang/std/num/uint16.ks`._

### Implements `Addable`

#### typealias `Output`

```kestrel
type Output = UInt16
```

_Defined in `lang/std/num/uint16.ks`._

#### typealias `Output`

```kestrel
type Output = UInt16
```

_Defined in `lang/std/num/uint16.ks`._

#### typealias `Output`

```kestrel
type Output = UInt16
```

_Defined in `lang/std/num/uint16.ks`._

#### typealias `Output`

```kestrel
type Output = UInt16
```

_Defined in `lang/std/num/uint16.ks`._

#### typealias `Output`

```kestrel
type Output = UInt16
```

_Defined in `lang/std/num/uint16.ks`._

#### typealias `Output`

```kestrel
type Output = UInt16
```

_Defined in `lang/std/num/uint16.ks`._

#### typealias `Output`

```kestrel
type Output = UInt16
```

_Defined in `lang/std/num/uint16.ks`._

#### typealias `Output`

```kestrel
type Output = UInt16
```

_Defined in `lang/std/num/uint16.ks`._

#### typealias `Output`

```kestrel
type Output = UInt16
```

_Defined in `lang/std/num/uint16.ks`._

#### typealias `Output`

```kestrel
type Output = UInt16
```

_Defined in `lang/std/num/uint16.ks`._

#### typealias `Output`

```kestrel
type Output = UInt16
```

_Defined in `lang/std/num/uint16.ks`._

#### typealias `Output`

```kestrel
type Output = Range[UInt16]
```

_Defined in `lang/std/num/uint16.ks`._

#### typealias `Output`

```kestrel
type Output = ClosedRange[UInt16]
```

_Defined in `lang/std/num/uint16.ks`._

#### function `add`

```kestrel
public func add(UInt16) -> UInt16
```

`self + other`, wrapping on overflow. Use `addChecked` to detect or
`addSaturating` to clamp.

_Defined in `lang/std/num/uint16.ks`._

#### field `zero`

```kestrel
public static var zero: UInt16 { get }
```

The additive identity, `0`.

_Defined in `lang/std/num/uint16.ks`._

### Implements `Subtractable`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/arithmetic.ks`._

#### function `subtract`

```kestrel
public func subtract(UInt16) -> UInt16
```

`self - other`, wrapping on overflow.

_Defined in `lang/std/num/uint16.ks`._

### Implements `Multipliable`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/arithmetic.ks`._

#### function `multiply`

```kestrel
public func multiply(UInt16) -> UInt16
```

`self * other`, wrapping on overflow.

_Defined in `lang/std/num/uint16.ks`._

#### field `one`

```kestrel
public static var one: UInt16 { get }
```

The multiplicative identity, `1`.

_Defined in `lang/std/num/uint16.ks`._

### Implements `Divisible`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/arithmetic.ks`._

#### function `divide`

```kestrel
public func divide(UInt16) -> UInt16
```

Truncating integer division (`self / other`). For signed types,
`minValue / -1` wraps; use `divideChecked` to detect.

##### Errors

Traps on division by zero (LLVM `udiv`/`sdiv` are UB on zero — the
process aborts before producing a result).

_Defined in `lang/std/num/uint16.ks`._

### Implements `Modulo`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/arithmetic.ks`._

#### function `modulo`

```kestrel
public func modulo(UInt16) -> UInt16
```

`self % other` — truncated remainder; the result has the sign of
`self` for signed types.

##### Errors

Traps on division by zero, like `divide`.

_Defined in `lang/std/num/uint16.ks`._

### Implements `BitwiseAnd`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/bitwise.ks`._

#### function `bitwiseAnd`

```kestrel
public func bitwiseAnd(UInt16) -> UInt16
```

Bitwise AND. `0b1010 & 0b1100 == 0b1000`.

_Defined in `lang/std/num/uint16.ks`._

### Implements `BitwiseOr`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/bitwise.ks`._

#### function `bitwiseOr`

```kestrel
public func bitwiseOr(UInt16) -> UInt16
```

Bitwise OR. `0b1010 | 0b1100 == 0b1110`.

_Defined in `lang/std/num/uint16.ks`._

### Implements `BitwiseXor`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/bitwise.ks`._

#### function `bitwiseXor`

```kestrel
public func bitwiseXor(UInt16) -> UInt16
```

Bitwise XOR. `0b1010 ^ 0b1100 == 0b0110`.

_Defined in `lang/std/num/uint16.ks`._

### Implements `BitwiseNot`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/bitwise.ks`._

#### function `bitwiseNot`

```kestrel
public func bitwiseNot() -> UInt16
```

Bitwise NOT — flips all bits. For signed types this is `-self - 1`.

_Defined in `lang/std/num/uint16.ks`._

### Implements `LeftShift`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/bitwise.ks`._

#### function `shiftLeft`

```kestrel
public func shiftLeft(by: lang.i64) -> UInt16
```

Left shift by `count`. Behavior is undefined when `count >= bitWidth`
— pre-mask the count if you can't guarantee the bound.

_Defined in `lang/std/num/uint16.ks`._

### Implements `RightShift`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/bitwise.ks`._

#### function `shiftRight`

```kestrel
public func shiftRight(by: lang.i64) -> UInt16
```

Right shift by `count`. Arithmetic (sign-extending) for signed types,
logical (zero-filling) for unsigned. Same `count` precondition as
`shiftLeft`.

_Defined in `lang/std/num/uint16.ks`._

### Implements `AddAssign`

#### function `addAssign`

```kestrel
public mutating func addAssign(UInt16)
```

`self += other`

_Defined in `lang/std/num/uint16.ks`._

### Implements `SubtractAssign`

#### function `subtractAssign`

```kestrel
public mutating func subtractAssign(UInt16)
```

`self -= other`

_Defined in `lang/std/num/uint16.ks`._

### Implements `MultiplyAssign`

#### function `multiplyAssign`

```kestrel
public mutating func multiplyAssign(UInt16)
```

`self *= other`

_Defined in `lang/std/num/uint16.ks`._

### Implements `DivideAssign`

#### function `divideAssign`

```kestrel
public mutating func divideAssign(UInt16)
```

`self /= other`

_Defined in `lang/std/num/uint16.ks`._

### Implements `ModuloAssign`

#### function `modAssign`

```kestrel
public mutating func modAssign(UInt16)
```

`self %= other`

_Defined in `lang/std/num/uint16.ks`._

### Implements `BitwiseAndAssign`

#### function `bitwiseAndAssign`

```kestrel
public mutating func bitwiseAndAssign(UInt16)
```

`self &= other`

_Defined in `lang/std/num/uint16.ks`._

### Implements `BitwiseOrAssign`

#### function `bitwiseOrAssign`

```kestrel
public mutating func bitwiseOrAssign(UInt16)
```

`self |= other`

_Defined in `lang/std/num/uint16.ks`._

### Implements `BitwiseXorAssign`

#### function `bitwiseXorAssign`

```kestrel
public mutating func bitwiseXorAssign(UInt16)
```

`self ^= other`

_Defined in `lang/std/num/uint16.ks`._

### Implements `LeftShiftAssign`

#### function `shiftLeftAssign`

```kestrel
public mutating func shiftLeftAssign(by: lang.i64)
```

`self <<= count`

_Defined in `lang/std/num/uint16.ks`._

### Implements `RightShiftAssign`

#### function `shiftRightAssign`

```kestrel
public mutating func shiftRightAssign(by: lang.i64)
```

`self >>= count`

_Defined in `lang/std/num/uint16.ks`._

### Implements `ExpressibleByIntLiteral`

#### initializer `Int Literal`

```kestrel
init(intLiteral: lang.i64)
```

Builds an instance from an integer literal.

_Defined in `lang/std/core/literals.ks`._

### Implements `Defaultable`

#### initializer `Default`

```kestrel
init()
```

Builds the default-valued instance.

_Defined in `lang/std/core/protocols.ks`._

### Implements `RangeConstructible`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/range.ks`._

#### function `exclusiveRange`

```kestrel
public func exclusiveRange(to: UInt16) -> Range[UInt16]
```

Builds a half-open range `self..<end`. Sugar for the `..<` operator.

_Defined in `lang/std/num/uint16.ks`._

### Implements `ClosedRangeConstructible`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/range.ks`._

#### function `inclusiveRange`

```kestrel
public func inclusiveRange(to: UInt16) -> ClosedRange[UInt16]
```

Builds a closed range `self..=end`. Sugar for the `..=` operator.

_Defined in `lang/std/num/uint16.ks`._

### Implements `Convertible`

#### initializer `From Source`

```kestrel
init(from: From)
```

Creates an instance from `value`.

_Defined in `lang/std/core/convertible.ks`._

## struct `UInt32`

```kestrel
public struct UInt32 { /* private fields */ }
```

A 32-bit unsigned integer.

UInt32 is the 32-bit member of the integer family. The same surface
area is provided across all widths; switch widths to trade range for memory
or to match an FFI ABI. Arithmetic wraps on overflow by default — use the
`*Checked` variants for overflow detection or `*Saturating` to clamp to
`minValue`/`maxValue`. The type is `FFISafe` and lays out as a single
`lang.i32` so it can cross C boundaries unchanged.

### Examples

```
let a: Int64 = 100;
let b = a + 50;        // 150
let c = a * 2;         // 200
let d = a.addChecked(Int64.maxValue);  // None (overflow detected)
```

```
// Bit twiddling
(0b1010).countOnes      // 2
(1).shiftLeft(by: 4)    // 16
(-1).leadingZeros       // 0  (all bits set)
```

### Representation

A single `lang.i32` field. No padding, no headers — bit-identical
to the corresponding C type.

_Defined in `lang/std/num/uint32.ks`._

### Members

#### initializer `Default`

```kestrel
public init()
```

Creates the zero value, satisfying `Defaultable`.

##### Examples

```
let n = Int64();   // 0
```

_Defined in `lang/std/num/uint32.ks`._

#### initializer `From Integer`

```kestrel
public init(from: Int8)
```

Converts from `Int8`. Narrowing conversions truncate the high
bits; signed→unsigned reinterprets the bit pattern.

_Defined in `lang/std/num/uint32.ks`._

#### initializer `From Integer`

```kestrel
public init(from: Int16)
```

Converts from `Int16`. Narrowing conversions truncate the high
bits; signed→unsigned reinterprets the bit pattern.

_Defined in `lang/std/num/uint32.ks`._

#### initializer `From Integer`

```kestrel
public init(from: Int32)
```

Converts from `Int32`. Narrowing conversions truncate the high
bits; signed→unsigned reinterprets the bit pattern.

_Defined in `lang/std/num/uint32.ks`._

#### initializer `From Integer`

```kestrel
public init(from: Int64)
```

Converts from `Int64`. Narrowing conversions truncate the high
bits; signed→unsigned reinterprets the bit pattern.

_Defined in `lang/std/num/uint32.ks`._

#### initializer `From Integer`

```kestrel
public init(from: UInt8)
```

Converts from `UInt8`. Narrowing conversions truncate the high
bits; signed→unsigned reinterprets the bit pattern.

_Defined in `lang/std/num/uint32.ks`._

#### initializer `From Integer`

```kestrel
public init(from: UInt16)
```

Converts from `UInt16`. Narrowing conversions truncate the high
bits; signed→unsigned reinterprets the bit pattern.

_Defined in `lang/std/num/uint32.ks`._

#### initializer `From Integer`

```kestrel
public init(from: UInt64)
```

Converts from `UInt64`. Narrowing conversions truncate the high
bits; signed→unsigned reinterprets the bit pattern.

_Defined in `lang/std/num/uint32.ks`._

#### initializer `From Raw`

```kestrel
init(raw: lang.i32)
```

Wraps an existing `lang.i32` without conversion. Internal
constructor used by intrinsics; not part of the public API.

_Defined in `lang/std/num/uint32.ks`._

#### initializer `Int Literal`

```kestrel
public init(intLiteral: lang.i64)
```

Compiler-emitted bridge that turns an integer literal into a UInt32.

You will rarely call this directly — write the literal and let the
`ExpressibleByIntLiteral` protocol pick it up. For widths smaller than
64 bits the literal is truncated with `lang.cast_i64_i32`.

##### Examples

```
let n: Int64 = 42;            // implicit
let m = Int64(intLiteral: 42);  // explicit
```

_Defined in `lang/std/num/uint32.ks`._

#### function `addChecked`

```kestrel
public func addChecked(UInt32) -> UInt32?
```

Wrapping addition that returns `None` on overflow. For unsigned types
overflow is detected via `result < self`.

_Defined in `lang/std/num/uint32.ks`._

#### function `addSaturating`

```kestrel
public func addSaturating(UInt32) -> UInt32
```

Addition that clamps to `maxValue` on overflow.

_Defined in `lang/std/num/uint32.ks`._

#### field `bitWidth`

```kestrel
public static var bitWidth: Int64 { get }
```

The width in bits (32). Useful for shift bounds and bit-walks.

_Defined in `lang/std/num/uint32.ks`._

#### field `byteSwapped`

```kestrel
public var byteSwapped: UInt32 { get }
```

Value with its byte order reversed. Use to convert between big- and
little-endian; lowered to a `bswap` intrinsic.

_Defined in `lang/std/num/uint32.ks`._

#### function `clamp`

```kestrel
public func clamp(UInt32, UInt32) -> UInt32
```

Clamps `self` into `[min, max]`. Caller is responsible for ensuring
`min <= max`; otherwise the result is undefined.

##### Examples

```
(5).clamp(min: 0, max: 10);    // 5
(-5).clamp(min: 0, max: 10);   // 0
(15).clamp(min: 0, max: 10);   // 10
```

_Defined in `lang/std/num/uint32.ks`._

#### field `countOnes`

```kestrel
public var countOnes: Int64 { get }
```

Population count — the number of `1` bits in the binary representation.

Lowered to a `popcount` intrinsic where the target supports it.

##### Examples

```
(0b1010).countOnes;  // 2
(0b1111).countOnes;  // 4
(0).countOnes;       // 0
```

_Defined in `lang/std/num/uint32.ks`._

#### field `countZeros`

```kestrel
public var countZeros: Int64 { get }
```

Complement of `countOnes`: equal to `bitWidth - countOnes`.

_Defined in `lang/std/num/uint32.ks`._

#### function `divideChecked`

```kestrel
public func divideChecked(UInt32) -> UInt32?
```

Division that returns `None` for divide-by-zero.

_Defined in `lang/std/num/uint32.ks`._

#### function `fromBytes`

```kestrel
public static func fromBytes(std.collections.Array[UInt8]) -> UInt32?
```

Reassembles a `UInt32` from 4 bytes in native (host) byte
order. Returns `None` if the input is not exactly 4 bytes long.

_Defined in `lang/std/num/uint32.ks`._

#### function `fromBytesBigEndian`

```kestrel
public static func fromBytesBigEndian(std.collections.Array[UInt8]) -> UInt32?
```

Reassembles a `UInt32` from 4 bytes in big-endian order.
Returns `None` if the input is not exactly 4 bytes long.

_Defined in `lang/std/num/uint32.ks`._

#### function `fromBytesLittleEndian`

```kestrel
public static func fromBytesLittleEndian(std.collections.Array[UInt8]) -> UInt32?
```

Reassembles a `UInt32` from 4 bytes in little-endian order.
Returns `None` if the input is not exactly 4 bytes long.

_Defined in `lang/std/num/uint32.ks`._

#### function `gcd`

```kestrel
public func gcd(UInt32) -> UInt32
```

Greatest common divisor via Euclidean algorithm. For signed types
the inputs are taken absolute first; the result is always non-negative.

##### Examples

```
(12).gcd(8);   // 4
(17).gcd(5);   // 1   (coprime)
(-12).gcd(8);  // 4
```

_Defined in `lang/std/num/uint32.ks`._

#### field `isNegative`

```kestrel
public var isNegative: Bool { get }
```

Always `false` — unsigned types cannot be negative.

_Defined in `lang/std/num/uint32.ks`._

#### field `isPositive`

```kestrel
public var isPositive: Bool { get }
```

True when `self > 0`.

_Defined in `lang/std/num/uint32.ks`._

#### field `isPowerOfTwo`

```kestrel
public var isPowerOfTwo: Bool { get }
```

True when the value is a positive power of two (`2^k` for `k >= 0`).

Zero and negatives are excluded. Cheap branchless test built on
`x & (x - 1) == 0`.

##### Examples

```
(1).isPowerOfTwo;   // true  (2^0)
(4).isPowerOfTwo;   // true  (2^2)
(3).isPowerOfTwo;   // false
(0).isPowerOfTwo;   // false
```

_Defined in `lang/std/num/uint32.ks`._

#### field `isZero`

```kestrel
public var isZero: Bool { get }
```

True when `self == 0`.

_Defined in `lang/std/num/uint32.ks`._

#### function `lcm`

```kestrel
public func lcm(UInt32) -> UInt32
```

Least common multiple, computed as `|self| / gcd(self, other) * |other|`
to avoid intermediate overflow. Returns zero if either input is zero.

##### Examples

```
(4).lcm(6);   // 12
(3).lcm(5);   // 15
(0).lcm(7);   // 0
```

_Defined in `lang/std/num/uint32.ks`._

#### field `leadingZeros`

```kestrel
public var leadingZeros: Int64 { get }
```

Number of leading zero bits, counting from the most-significant end.

For zero, returns `bitWidth`.

##### Examples

```
(1).leadingZeros;   // bitWidth - 1
(0).leadingZeros;   // bitWidth
```

_Defined in `lang/std/num/uint32.ks`._

#### field `maxValue`

```kestrel
public static var maxValue: UInt32 { get }
```

The largest representable value.
This is 2^32 - 1 (4_294_967_295).

_Defined in `lang/std/num/uint32.ks`._

#### field `minValue`

```kestrel
public static var minValue: UInt32 { get }
```

The smallest representable value.
This is always 0 for unsigned types.
Note that for signed types `minValue.negate()` overflows back to
itself; use `negateChecked()` if you need to detect that.

_Defined in `lang/std/num/uint32.ks`._

#### function `multiplyChecked`

```kestrel
public func multiplyChecked(UInt32) -> UInt32?
```

Wrapping multiplication that returns `None` on overflow. Implemented
by multiplying then dividing back.

_Defined in `lang/std/num/uint32.ks`._

#### function `multiplySaturating`

```kestrel
public func multiplySaturating(UInt32) -> UInt32
```

Multiplication that clamps to `maxValue` on overflow.

_Defined in `lang/std/num/uint32.ks`._

#### function `parse`

```kestrel
public static func parse(String) -> UInt32?
```

Parses a base-10 unsigned integer literal, optionally prefixed
with `+`. A leading `-` is rejected. Returns `None` for an empty
string, a non-digit character, or a value that does not fit in
`UInt32`.

##### Examples

```
UInt32.parse(string: "42");   // Some(42)
UInt32.parse(string: "-1");   // None  (no sign for unsigned)
UInt32.parse(string: "");     // None
```

_Defined in `lang/std/num/uint32.ks`._

#### function `parse`

```kestrel
public static func parse(String, Int64) -> UInt32?
```

Parses an unsigned integer in `radix` (base 2–36 inclusive). Letters
a–z are case-insensitive and represent digit values 10–35. A
leading `+` is allowed but a leading `-` is rejected. Returns
`None` for an out-of-range radix, an empty string, an
unrecognised digit, or a value that overflows `UInt32`.

##### Examples

```
UInt32.parse(string: "ff", radix: 16);     // Some(255 if it fits, else None)
UInt32.parse(string: "101010", radix: 2);  // Some(42)
```

_Defined in `lang/std/num/uint32.ks`._

#### function `pow`

```kestrel
public func pow(Int64) -> UInt32
```

Raises `self` to `exponent` via binary exponentiation. Wraps on
overflow. Negative exponents return zero (integer truncation of
the would-be fraction).

##### Examples

```
(2).pow(10);  // 1024
(3).pow(4);   // 81
(5).pow(-1);  // 0
```

_Defined in `lang/std/num/uint32.ks`._

#### field `raw`

```kestrel
public var raw: lang.i32
```

The underlying primitive `lang.i32` value. Exposed for FFI
and intrinsic use; prefer the typed surface for everything else.

_Defined in `lang/std/num/uint32.ks`._

#### function `rotateLeft`

```kestrel
public func rotateLeft(by: Int64) -> UInt32
```

Rotates bits left by `count`, modulo `bitWidth`. Bits shifted past the
MSB re-enter at the LSB.

_Defined in `lang/std/num/uint32.ks`._

#### function `rotateRight`

```kestrel
public func rotateRight(by: Int64) -> UInt32
```

Rotates bits right by `count`, modulo `bitWidth`. Mirror of
`rotateLeft`.

_Defined in `lang/std/num/uint32.ks`._

#### field `sign`

```kestrel
public var sign: UInt32 { get }
```

Sign as a `UInt32`: `0` for zero, `1` otherwise (unsigned types
have no negative values).

_Defined in `lang/std/num/uint32.ks`._

#### function `subtractChecked`

```kestrel
public func subtractChecked(UInt32) -> UInt32?
```

Subtraction that returns `None` on underflow (`other > self`).

_Defined in `lang/std/num/uint32.ks`._

#### function `subtractSaturating`

```kestrel
public func subtractSaturating(UInt32) -> UInt32
```

Subtraction that clamps to `0` on underflow (unsigned types cannot
represent negative results).

_Defined in `lang/std/num/uint32.ks`._

#### function `toBytes`

```kestrel
public func toBytes() -> std.collections.Array[UInt8]
```

Splits this integer into 4 bytes in *native* (host) byte order.
Use `toBytesBigEndian` / `toBytesLittleEndian` when serialising for
a fixed wire format.

##### Examples

```
let bytes = UInt32.maxValue.toBytes();   // 4 bytes, host order
```

_Defined in `lang/std/num/uint32.ks`._

#### function `toBytesBigEndian`

```kestrel
public func toBytesBigEndian() -> std.collections.Array[UInt8]
```

Splits this integer into 4 bytes in big-endian order (most
significant byte first — i.e. network byte order).

_Defined in `lang/std/num/uint32.ks`._

#### function `toBytesLittleEndian`

```kestrel
public func toBytesLittleEndian() -> std.collections.Array[UInt8]
```

Splits this integer into 4 bytes in little-endian order (least
significant byte first).

_Defined in `lang/std/num/uint32.ks`._

#### field `trailingZeros`

```kestrel
public var trailingZeros: Int64 { get }
```

Number of trailing zero bits. Equal to `log2(self & -self)` for non-zero
values; returns `bitWidth` for zero. Useful for finding the largest
power of two dividing the value.

_Defined in `lang/std/num/uint32.ks`._

### Implements `Steppable`

#### function `predecessor`

```kestrel
public func predecessor() -> UInt32
```

Predecessor — `self - 1`. Wraps at `minValue`.

_Defined in `lang/std/num/uint32.ks`._

#### function `successor`

```kestrel
public func successor() -> UInt32
```

Successor — `self + 1`. Wraps at `maxValue`. Used by `for-in` over
integer ranges.

_Defined in `lang/std/num/uint32.ks`._

### Implements `Comparable`

#### function `compare`

```kestrel
public func compare(UInt32) -> Ordering
```

Three-way comparison returning an `Ordering`. Signed types compare
using two's-complement ordering; unsigned types use natural ordering.

##### Examples

```
(1).compare(other: 2);   // .Less
(2).compare(other: 2);   // .Equal
(3).compare(other: 2);   // .Greater
```

_Defined in `lang/std/num/uint32.ks`._

### Implements `Equatable`

#### function `equals`

```kestrel
public func equals(UInt32) -> Bool
```

Bit-for-bit equality. Backs the `==` operator.

##### Examples

```
(42).equals(other: 42);  // true
42 == 42;                // true
```

_Defined in `lang/std/num/uint32.ks`._

### Implements `Matchable`

#### function `matches`

```kestrel
public func matches(UInt32) -> Bool
```

Pattern-matching hook for `Matchable`. Identical to `equals`.

_Defined in `lang/std/num/uint32.ks`._

### Implements `Formattable`

#### function `format`

```kestrel
public func format(FormatOptions) -> String
```

Renders the integer to a `String`, honouring the supplied
`FormatOptions`. Implements the `Formattable` protocol.

Recognised options:
- `radix` — base in `[2, 36]`; out-of-range values fall back to 10.
- `width` — minimum output width; shorter values are padded.
- `fill` / `alignment` — padding character and side.
- `sign` — `.Negative` (default), `.Always`, or `.Space`.
- `uppercase` — uppercase hex digits.
- `alternate` — emit the `0b` / `0o` / `0x` prefix.

##### Examples

```
(42).format();                                           // "42"
(255).format(options: .{radix: 16});                     // "ff"
(255).format(options: .{radix: 16, uppercase: true});    // "FF"
(255).format(options: .{radix: 16, alternate: true});    // "0xff"
(42).format(options: .{radix: 2, alternate: true});      // "0b101010"
(42).format(options: .{width: .Some(5), fill: '0'});     // "00042"
(-42).format(options: .{sign: .Always});                 // "-42"
```

_Defined in `lang/std/num/uint32.ks`._

### Implements `Hash`

#### function `hash`

```kestrel
public func hash[H](into: mutating H) where H: Hasher
```

Feeds the raw bytes of this value into `hasher`. Endianness-agnostic
only within a single process — do not persist hashes across builds.

_Defined in `lang/std/num/uint32.ks`._

### Implements `Addable`

#### typealias `Output`

```kestrel
type Output = UInt32
```

_Defined in `lang/std/num/uint32.ks`._

#### typealias `Output`

```kestrel
type Output = UInt32
```

_Defined in `lang/std/num/uint32.ks`._

#### typealias `Output`

```kestrel
type Output = UInt32
```

_Defined in `lang/std/num/uint32.ks`._

#### typealias `Output`

```kestrel
type Output = UInt32
```

_Defined in `lang/std/num/uint32.ks`._

#### typealias `Output`

```kestrel
type Output = UInt32
```

_Defined in `lang/std/num/uint32.ks`._

#### typealias `Output`

```kestrel
type Output = UInt32
```

_Defined in `lang/std/num/uint32.ks`._

#### typealias `Output`

```kestrel
type Output = UInt32
```

_Defined in `lang/std/num/uint32.ks`._

#### typealias `Output`

```kestrel
type Output = UInt32
```

_Defined in `lang/std/num/uint32.ks`._

#### typealias `Output`

```kestrel
type Output = UInt32
```

_Defined in `lang/std/num/uint32.ks`._

#### typealias `Output`

```kestrel
type Output = UInt32
```

_Defined in `lang/std/num/uint32.ks`._

#### typealias `Output`

```kestrel
type Output = UInt32
```

_Defined in `lang/std/num/uint32.ks`._

#### typealias `Output`

```kestrel
type Output = Range[UInt32]
```

_Defined in `lang/std/num/uint32.ks`._

#### typealias `Output`

```kestrel
type Output = ClosedRange[UInt32]
```

_Defined in `lang/std/num/uint32.ks`._

#### function `add`

```kestrel
public func add(UInt32) -> UInt32
```

`self + other`, wrapping on overflow. Use `addChecked` to detect or
`addSaturating` to clamp.

_Defined in `lang/std/num/uint32.ks`._

#### field `zero`

```kestrel
public static var zero: UInt32 { get }
```

The additive identity, `0`.

_Defined in `lang/std/num/uint32.ks`._

### Implements `Subtractable`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/arithmetic.ks`._

#### function `subtract`

```kestrel
public func subtract(UInt32) -> UInt32
```

`self - other`, wrapping on overflow.

_Defined in `lang/std/num/uint32.ks`._

### Implements `Multipliable`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/arithmetic.ks`._

#### function `multiply`

```kestrel
public func multiply(UInt32) -> UInt32
```

`self * other`, wrapping on overflow.

_Defined in `lang/std/num/uint32.ks`._

#### field `one`

```kestrel
public static var one: UInt32 { get }
```

The multiplicative identity, `1`.

_Defined in `lang/std/num/uint32.ks`._

### Implements `Divisible`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/arithmetic.ks`._

#### function `divide`

```kestrel
public func divide(UInt32) -> UInt32
```

Truncating integer division (`self / other`). For signed types,
`minValue / -1` wraps; use `divideChecked` to detect.

##### Errors

Traps on division by zero (LLVM `udiv`/`sdiv` are UB on zero — the
process aborts before producing a result).

_Defined in `lang/std/num/uint32.ks`._

### Implements `Modulo`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/arithmetic.ks`._

#### function `modulo`

```kestrel
public func modulo(UInt32) -> UInt32
```

`self % other` — truncated remainder; the result has the sign of
`self` for signed types.

##### Errors

Traps on division by zero, like `divide`.

_Defined in `lang/std/num/uint32.ks`._

### Implements `BitwiseAnd`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/bitwise.ks`._

#### function `bitwiseAnd`

```kestrel
public func bitwiseAnd(UInt32) -> UInt32
```

Bitwise AND. `0b1010 & 0b1100 == 0b1000`.

_Defined in `lang/std/num/uint32.ks`._

### Implements `BitwiseOr`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/bitwise.ks`._

#### function `bitwiseOr`

```kestrel
public func bitwiseOr(UInt32) -> UInt32
```

Bitwise OR. `0b1010 | 0b1100 == 0b1110`.

_Defined in `lang/std/num/uint32.ks`._

### Implements `BitwiseXor`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/bitwise.ks`._

#### function `bitwiseXor`

```kestrel
public func bitwiseXor(UInt32) -> UInt32
```

Bitwise XOR. `0b1010 ^ 0b1100 == 0b0110`.

_Defined in `lang/std/num/uint32.ks`._

### Implements `BitwiseNot`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/bitwise.ks`._

#### function `bitwiseNot`

```kestrel
public func bitwiseNot() -> UInt32
```

Bitwise NOT — flips all bits. For signed types this is `-self - 1`.

_Defined in `lang/std/num/uint32.ks`._

### Implements `LeftShift`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/bitwise.ks`._

#### function `shiftLeft`

```kestrel
public func shiftLeft(by: lang.i64) -> UInt32
```

Left shift by `count`. Behavior is undefined when `count >= bitWidth`
— pre-mask the count if you can't guarantee the bound.

_Defined in `lang/std/num/uint32.ks`._

### Implements `RightShift`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/bitwise.ks`._

#### function `shiftRight`

```kestrel
public func shiftRight(by: lang.i64) -> UInt32
```

Right shift by `count`. Arithmetic (sign-extending) for signed types,
logical (zero-filling) for unsigned. Same `count` precondition as
`shiftLeft`.

_Defined in `lang/std/num/uint32.ks`._

### Implements `AddAssign`

#### function `addAssign`

```kestrel
public mutating func addAssign(UInt32)
```

`self += other`

_Defined in `lang/std/num/uint32.ks`._

### Implements `SubtractAssign`

#### function `subtractAssign`

```kestrel
public mutating func subtractAssign(UInt32)
```

`self -= other`

_Defined in `lang/std/num/uint32.ks`._

### Implements `MultiplyAssign`

#### function `multiplyAssign`

```kestrel
public mutating func multiplyAssign(UInt32)
```

`self *= other`

_Defined in `lang/std/num/uint32.ks`._

### Implements `DivideAssign`

#### function `divideAssign`

```kestrel
public mutating func divideAssign(UInt32)
```

`self /= other`

_Defined in `lang/std/num/uint32.ks`._

### Implements `ModuloAssign`

#### function `modAssign`

```kestrel
public mutating func modAssign(UInt32)
```

`self %= other`

_Defined in `lang/std/num/uint32.ks`._

### Implements `BitwiseAndAssign`

#### function `bitwiseAndAssign`

```kestrel
public mutating func bitwiseAndAssign(UInt32)
```

`self &= other`

_Defined in `lang/std/num/uint32.ks`._

### Implements `BitwiseOrAssign`

#### function `bitwiseOrAssign`

```kestrel
public mutating func bitwiseOrAssign(UInt32)
```

`self |= other`

_Defined in `lang/std/num/uint32.ks`._

### Implements `BitwiseXorAssign`

#### function `bitwiseXorAssign`

```kestrel
public mutating func bitwiseXorAssign(UInt32)
```

`self ^= other`

_Defined in `lang/std/num/uint32.ks`._

### Implements `LeftShiftAssign`

#### function `shiftLeftAssign`

```kestrel
public mutating func shiftLeftAssign(by: lang.i64)
```

`self <<= count`

_Defined in `lang/std/num/uint32.ks`._

### Implements `RightShiftAssign`

#### function `shiftRightAssign`

```kestrel
public mutating func shiftRightAssign(by: lang.i64)
```

`self >>= count`

_Defined in `lang/std/num/uint32.ks`._

### Implements `ExpressibleByIntLiteral`

#### initializer `Int Literal`

```kestrel
init(intLiteral: lang.i64)
```

Builds an instance from an integer literal.

_Defined in `lang/std/core/literals.ks`._

### Implements `Defaultable`

#### initializer `Default`

```kestrel
init()
```

Builds the default-valued instance.

_Defined in `lang/std/core/protocols.ks`._

### Implements `RangeConstructible`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/range.ks`._

#### function `exclusiveRange`

```kestrel
public func exclusiveRange(to: UInt32) -> Range[UInt32]
```

Builds a half-open range `self..<end`. Sugar for the `..<` operator.

_Defined in `lang/std/num/uint32.ks`._

### Implements `ClosedRangeConstructible`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/range.ks`._

#### function `inclusiveRange`

```kestrel
public func inclusiveRange(to: UInt32) -> ClosedRange[UInt32]
```

Builds a closed range `self..=end`. Sugar for the `..=` operator.

_Defined in `lang/std/num/uint32.ks`._

### Implements `Convertible`

#### initializer `From Source`

```kestrel
init(from: From)
```

Creates an instance from `value`.

_Defined in `lang/std/core/convertible.ks`._

## struct `UInt64`

```kestrel
public struct UInt64 { /* private fields */ }
```

A 64-bit unsigned integer.

UInt64 is the 64-bit member of the integer family. The same surface
area is provided across all widths; switch widths to trade range for memory
or to match an FFI ABI. Arithmetic wraps on overflow by default — use the
`*Checked` variants for overflow detection or `*Saturating` to clamp to
`minValue`/`maxValue`. The type is `FFISafe` and lays out as a single
`lang.i64` so it can cross C boundaries unchanged.

### Examples

```
let a: Int64 = 100;
let b = a + 50;        // 150
let c = a * 2;         // 200
let d = a.addChecked(Int64.maxValue);  // None (overflow detected)
```

```
// Bit twiddling
(0b1010).countOnes      // 2
(1).shiftLeft(by: 4)    // 16
(-1).leadingZeros       // 0  (all bits set)
```

### Representation

A single `lang.i64` field. No padding, no headers — bit-identical
to the corresponding C type.

_Defined in `lang/std/num/uint64.ks`._

### Members

#### initializer `Default`

```kestrel
public init()
```

Creates the zero value, satisfying `Defaultable`.

##### Examples

```
let n = Int64();   // 0
```

_Defined in `lang/std/num/uint64.ks`._

#### initializer `From Integer`

```kestrel
public init(from: Int8)
```

Converts from `Int8`. Narrowing conversions truncate the high
bits; signed→unsigned reinterprets the bit pattern.

_Defined in `lang/std/num/uint64.ks`._

#### initializer `From Integer`

```kestrel
public init(from: Int16)
```

Converts from `Int16`. Narrowing conversions truncate the high
bits; signed→unsigned reinterprets the bit pattern.

_Defined in `lang/std/num/uint64.ks`._

#### initializer `From Integer`

```kestrel
public init(from: Int32)
```

Converts from `Int32`. Narrowing conversions truncate the high
bits; signed→unsigned reinterprets the bit pattern.

_Defined in `lang/std/num/uint64.ks`._

#### initializer `From Integer`

```kestrel
public init(from: Int64)
```

Converts from `Int64`. Narrowing conversions truncate the high
bits; signed→unsigned reinterprets the bit pattern.

_Defined in `lang/std/num/uint64.ks`._

#### initializer `From Integer`

```kestrel
public init(from: UInt8)
```

Converts from `UInt8`. Narrowing conversions truncate the high
bits; signed→unsigned reinterprets the bit pattern.

_Defined in `lang/std/num/uint64.ks`._

#### initializer `From Integer`

```kestrel
public init(from: UInt16)
```

Converts from `UInt16`. Narrowing conversions truncate the high
bits; signed→unsigned reinterprets the bit pattern.

_Defined in `lang/std/num/uint64.ks`._

#### initializer `From Integer`

```kestrel
public init(from: UInt32)
```

Converts from `UInt32`. Narrowing conversions truncate the high
bits; signed→unsigned reinterprets the bit pattern.

_Defined in `lang/std/num/uint64.ks`._

#### initializer `From Raw`

```kestrel
init(raw: lang.i64)
```

Wraps an existing `lang.i64` without conversion. Internal
constructor used by intrinsics; not part of the public API.

_Defined in `lang/std/num/uint64.ks`._

#### initializer `Int Literal`

```kestrel
public init(intLiteral: lang.i64)
```

Compiler-emitted bridge that turns an integer literal into a UInt64.

You will rarely call this directly — write the literal and let the
`ExpressibleByIntLiteral` protocol pick it up. For widths smaller than
64 bits the literal is truncated with `lang.cast_i64_i64`.

##### Examples

```
let n: Int64 = 42;            // implicit
let m = Int64(intLiteral: 42);  // explicit
```

_Defined in `lang/std/num/uint64.ks`._

#### function `addChecked`

```kestrel
public func addChecked(UInt64) -> UInt64?
```

Wrapping addition that returns `None` on overflow. For unsigned types
overflow is detected via `result < self`.

_Defined in `lang/std/num/uint64.ks`._

#### function `addSaturating`

```kestrel
public func addSaturating(UInt64) -> UInt64
```

Addition that clamps to `maxValue` on overflow.

_Defined in `lang/std/num/uint64.ks`._

#### field `bitWidth`

```kestrel
public static var bitWidth: Int64 { get }
```

The width in bits (64). Useful for shift bounds and bit-walks.

_Defined in `lang/std/num/uint64.ks`._

#### field `byteSwapped`

```kestrel
public var byteSwapped: UInt64 { get }
```

Value with its byte order reversed. Use to convert between big- and
little-endian; lowered to a `bswap` intrinsic.

_Defined in `lang/std/num/uint64.ks`._

#### function `clamp`

```kestrel
public func clamp(UInt64, UInt64) -> UInt64
```

Clamps `self` into `[min, max]`. Caller is responsible for ensuring
`min <= max`; otherwise the result is undefined.

##### Examples

```
(5).clamp(min: 0, max: 10);    // 5
(-5).clamp(min: 0, max: 10);   // 0
(15).clamp(min: 0, max: 10);   // 10
```

_Defined in `lang/std/num/uint64.ks`._

#### field `countOnes`

```kestrel
public var countOnes: Int64 { get }
```

Population count — the number of `1` bits in the binary representation.

Lowered to a `popcount` intrinsic where the target supports it.

##### Examples

```
(0b1010).countOnes;  // 2
(0b1111).countOnes;  // 4
(0).countOnes;       // 0
```

_Defined in `lang/std/num/uint64.ks`._

#### field `countZeros`

```kestrel
public var countZeros: Int64 { get }
```

Complement of `countOnes`: equal to `bitWidth - countOnes`.

_Defined in `lang/std/num/uint64.ks`._

#### function `divideChecked`

```kestrel
public func divideChecked(UInt64) -> UInt64?
```

Division that returns `None` for divide-by-zero.

_Defined in `lang/std/num/uint64.ks`._

#### function `fromBytes`

```kestrel
public static func fromBytes(std.collections.Array[UInt8]) -> UInt64?
```

Reassembles a `UInt64` from 8 bytes in native (host) byte
order. Returns `None` if the input is not exactly 8 bytes long.

_Defined in `lang/std/num/uint64.ks`._

#### function `fromBytesBigEndian`

```kestrel
public static func fromBytesBigEndian(std.collections.Array[UInt8]) -> UInt64?
```

Reassembles a `UInt64` from 8 bytes in big-endian order.
Returns `None` if the input is not exactly 8 bytes long.

_Defined in `lang/std/num/uint64.ks`._

#### function `fromBytesLittleEndian`

```kestrel
public static func fromBytesLittleEndian(std.collections.Array[UInt8]) -> UInt64?
```

Reassembles a `UInt64` from 8 bytes in little-endian order.
Returns `None` if the input is not exactly 8 bytes long.

_Defined in `lang/std/num/uint64.ks`._

#### function `gcd`

```kestrel
public func gcd(UInt64) -> UInt64
```

Greatest common divisor via Euclidean algorithm. For signed types
the inputs are taken absolute first; the result is always non-negative.

##### Examples

```
(12).gcd(8);   // 4
(17).gcd(5);   // 1   (coprime)
(-12).gcd(8);  // 4
```

_Defined in `lang/std/num/uint64.ks`._

#### field `isNegative`

```kestrel
public var isNegative: Bool { get }
```

Always `false` — unsigned types cannot be negative.

_Defined in `lang/std/num/uint64.ks`._

#### field `isPositive`

```kestrel
public var isPositive: Bool { get }
```

True when `self > 0`.

_Defined in `lang/std/num/uint64.ks`._

#### field `isPowerOfTwo`

```kestrel
public var isPowerOfTwo: Bool { get }
```

True when the value is a positive power of two (`2^k` for `k >= 0`).

Zero and negatives are excluded. Cheap branchless test built on
`x & (x - 1) == 0`.

##### Examples

```
(1).isPowerOfTwo;   // true  (2^0)
(4).isPowerOfTwo;   // true  (2^2)
(3).isPowerOfTwo;   // false
(0).isPowerOfTwo;   // false
```

_Defined in `lang/std/num/uint64.ks`._

#### field `isZero`

```kestrel
public var isZero: Bool { get }
```

True when `self == 0`.

_Defined in `lang/std/num/uint64.ks`._

#### function `lcm`

```kestrel
public func lcm(UInt64) -> UInt64
```

Least common multiple, computed as `|self| / gcd(self, other) * |other|`
to avoid intermediate overflow. Returns zero if either input is zero.

##### Examples

```
(4).lcm(6);   // 12
(3).lcm(5);   // 15
(0).lcm(7);   // 0
```

_Defined in `lang/std/num/uint64.ks`._

#### field `leadingZeros`

```kestrel
public var leadingZeros: Int64 { get }
```

Number of leading zero bits, counting from the most-significant end.

For zero, returns `bitWidth`.

##### Examples

```
(1).leadingZeros;   // bitWidth - 1
(0).leadingZeros;   // bitWidth
```

_Defined in `lang/std/num/uint64.ks`._

#### field `maxValue`

```kestrel
public static var maxValue: UInt64 { get }
```

The largest representable value.
This is 2^64 - 1 (18_446_744_073_709_551_615).

_Defined in `lang/std/num/uint64.ks`._

#### field `minValue`

```kestrel
public static var minValue: UInt64 { get }
```

The smallest representable value.
This is always 0 for unsigned types.
Note that for signed types `minValue.negate()` overflows back to
itself; use `negateChecked()` if you need to detect that.

_Defined in `lang/std/num/uint64.ks`._

#### function `multiplyChecked`

```kestrel
public func multiplyChecked(UInt64) -> UInt64?
```

Wrapping multiplication that returns `None` on overflow. Implemented
by multiplying then dividing back.

_Defined in `lang/std/num/uint64.ks`._

#### function `multiplySaturating`

```kestrel
public func multiplySaturating(UInt64) -> UInt64
```

Multiplication that clamps to `maxValue` on overflow.

_Defined in `lang/std/num/uint64.ks`._

#### function `parse`

```kestrel
public static func parse(String) -> UInt64?
```

Parses a base-10 unsigned integer literal, optionally prefixed
with `+`. A leading `-` is rejected. Returns `None` for an empty
string, a non-digit character, or a value that does not fit in
`UInt64`.

##### Examples

```
UInt64.parse(string: "42");   // Some(42)
UInt64.parse(string: "-1");   // None  (no sign for unsigned)
UInt64.parse(string: "");     // None
```

_Defined in `lang/std/num/uint64.ks`._

#### function `parse`

```kestrel
public static func parse(String, Int64) -> UInt64?
```

Parses an unsigned integer in `radix` (base 2–36 inclusive). Letters
a–z are case-insensitive and represent digit values 10–35. A
leading `+` is allowed but a leading `-` is rejected. Returns
`None` for an out-of-range radix, an empty string, an
unrecognised digit, or a value that overflows `UInt64`.

##### Examples

```
UInt64.parse(string: "ff", radix: 16);     // Some(255 if it fits, else None)
UInt64.parse(string: "101010", radix: 2);  // Some(42)
```

_Defined in `lang/std/num/uint64.ks`._

#### function `pow`

```kestrel
public func pow(Int64) -> UInt64
```

Raises `self` to `exponent` via binary exponentiation. Wraps on
overflow. Negative exponents return zero (integer truncation of
the would-be fraction).

##### Examples

```
(2).pow(10);  // 1024
(3).pow(4);   // 81
(5).pow(-1);  // 0
```

_Defined in `lang/std/num/uint64.ks`._

#### field `raw`

```kestrel
public var raw: lang.i64
```

The underlying primitive `lang.i64` value. Exposed for FFI
and intrinsic use; prefer the typed surface for everything else.

_Defined in `lang/std/num/uint64.ks`._

#### function `rotateLeft`

```kestrel
public func rotateLeft(by: Int64) -> UInt64
```

Rotates bits left by `count`, modulo `bitWidth`. Bits shifted past the
MSB re-enter at the LSB.

_Defined in `lang/std/num/uint64.ks`._

#### function `rotateRight`

```kestrel
public func rotateRight(by: Int64) -> UInt64
```

Rotates bits right by `count`, modulo `bitWidth`. Mirror of
`rotateLeft`.

_Defined in `lang/std/num/uint64.ks`._

#### field `sign`

```kestrel
public var sign: UInt64 { get }
```

Sign as a `UInt64`: `0` for zero, `1` otherwise (unsigned types
have no negative values).

_Defined in `lang/std/num/uint64.ks`._

#### function `subtractChecked`

```kestrel
public func subtractChecked(UInt64) -> UInt64?
```

Subtraction that returns `None` on underflow (`other > self`).

_Defined in `lang/std/num/uint64.ks`._

#### function `subtractSaturating`

```kestrel
public func subtractSaturating(UInt64) -> UInt64
```

Subtraction that clamps to `0` on underflow (unsigned types cannot
represent negative results).

_Defined in `lang/std/num/uint64.ks`._

#### function `toBytes`

```kestrel
public func toBytes() -> std.collections.Array[UInt8]
```

Splits this integer into 8 bytes in *native* (host) byte order.
Use `toBytesBigEndian` / `toBytesLittleEndian` when serialising for
a fixed wire format.

##### Examples

```
let bytes = UInt64.maxValue.toBytes();   // 8 bytes, host order
```

_Defined in `lang/std/num/uint64.ks`._

#### function `toBytesBigEndian`

```kestrel
public func toBytesBigEndian() -> std.collections.Array[UInt8]
```

Splits this integer into 8 bytes in big-endian order (most
significant byte first — i.e. network byte order).

_Defined in `lang/std/num/uint64.ks`._

#### function `toBytesLittleEndian`

```kestrel
public func toBytesLittleEndian() -> std.collections.Array[UInt8]
```

Splits this integer into 8 bytes in little-endian order (least
significant byte first).

_Defined in `lang/std/num/uint64.ks`._

#### field `trailingZeros`

```kestrel
public var trailingZeros: Int64 { get }
```

Number of trailing zero bits. Equal to `log2(self & -self)` for non-zero
values; returns `bitWidth` for zero. Useful for finding the largest
power of two dividing the value.

_Defined in `lang/std/num/uint64.ks`._

### Implements `Steppable`

#### function `predecessor`

```kestrel
public func predecessor() -> UInt64
```

Predecessor — `self - 1`. Wraps at `minValue`.

_Defined in `lang/std/num/uint64.ks`._

#### function `successor`

```kestrel
public func successor() -> UInt64
```

Successor — `self + 1`. Wraps at `maxValue`. Used by `for-in` over
integer ranges.

_Defined in `lang/std/num/uint64.ks`._

### Implements `Comparable`

#### function `compare`

```kestrel
public func compare(UInt64) -> Ordering
```

Three-way comparison returning an `Ordering`. Signed types compare
using two's-complement ordering; unsigned types use natural ordering.

##### Examples

```
(1).compare(other: 2);   // .Less
(2).compare(other: 2);   // .Equal
(3).compare(other: 2);   // .Greater
```

_Defined in `lang/std/num/uint64.ks`._

### Implements `Equatable`

#### function `equals`

```kestrel
public func equals(UInt64) -> Bool
```

Bit-for-bit equality. Backs the `==` operator.

##### Examples

```
(42).equals(other: 42);  // true
42 == 42;                // true
```

_Defined in `lang/std/num/uint64.ks`._

### Implements `Matchable`

#### function `matches`

```kestrel
public func matches(UInt64) -> Bool
```

Pattern-matching hook for `Matchable`. Identical to `equals`.

_Defined in `lang/std/num/uint64.ks`._

### Implements `Formattable`

#### function `format`

```kestrel
public func format(FormatOptions) -> String
```

Renders the integer to a `String`, honouring the supplied
`FormatOptions`. Implements the `Formattable` protocol.

Recognised options:
- `radix` — base in `[2, 36]`; out-of-range values fall back to 10.
- `width` — minimum output width; shorter values are padded.
- `fill` / `alignment` — padding character and side.
- `sign` — `.Negative` (default), `.Always`, or `.Space`.
- `uppercase` — uppercase hex digits.
- `alternate` — emit the `0b` / `0o` / `0x` prefix.

##### Examples

```
(42).format();                                           // "42"
(255).format(options: .{radix: 16});                     // "ff"
(255).format(options: .{radix: 16, uppercase: true});    // "FF"
(255).format(options: .{radix: 16, alternate: true});    // "0xff"
(42).format(options: .{radix: 2, alternate: true});      // "0b101010"
(42).format(options: .{width: .Some(5), fill: '0'});     // "00042"
(-42).format(options: .{sign: .Always});                 // "-42"
```

_Defined in `lang/std/num/uint64.ks`._

### Implements `Hash`

#### function `hash`

```kestrel
public func hash[H](into: mutating H) where H: Hasher
```

Feeds the raw bytes of this value into `hasher`. Endianness-agnostic
only within a single process — do not persist hashes across builds.

_Defined in `lang/std/num/uint64.ks`._

### Implements `Addable`

#### typealias `Output`

```kestrel
type Output = UInt64
```

_Defined in `lang/std/num/uint64.ks`._

#### typealias `Output`

```kestrel
type Output = UInt64
```

_Defined in `lang/std/num/uint64.ks`._

#### typealias `Output`

```kestrel
type Output = UInt64
```

_Defined in `lang/std/num/uint64.ks`._

#### typealias `Output`

```kestrel
type Output = UInt64
```

_Defined in `lang/std/num/uint64.ks`._

#### typealias `Output`

```kestrel
type Output = UInt64
```

_Defined in `lang/std/num/uint64.ks`._

#### typealias `Output`

```kestrel
type Output = UInt64
```

_Defined in `lang/std/num/uint64.ks`._

#### typealias `Output`

```kestrel
type Output = UInt64
```

_Defined in `lang/std/num/uint64.ks`._

#### typealias `Output`

```kestrel
type Output = UInt64
```

_Defined in `lang/std/num/uint64.ks`._

#### typealias `Output`

```kestrel
type Output = UInt64
```

_Defined in `lang/std/num/uint64.ks`._

#### typealias `Output`

```kestrel
type Output = UInt64
```

_Defined in `lang/std/num/uint64.ks`._

#### typealias `Output`

```kestrel
type Output = UInt64
```

_Defined in `lang/std/num/uint64.ks`._

#### typealias `Output`

```kestrel
type Output = Range[UInt64]
```

_Defined in `lang/std/num/uint64.ks`._

#### typealias `Output`

```kestrel
type Output = ClosedRange[UInt64]
```

_Defined in `lang/std/num/uint64.ks`._

#### function `add`

```kestrel
public func add(UInt64) -> UInt64
```

`self + other`, wrapping on overflow. Use `addChecked` to detect or
`addSaturating` to clamp.

_Defined in `lang/std/num/uint64.ks`._

#### field `zero`

```kestrel
public static var zero: UInt64 { get }
```

The additive identity, `0`.

_Defined in `lang/std/num/uint64.ks`._

### Implements `Subtractable`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/arithmetic.ks`._

#### function `subtract`

```kestrel
public func subtract(UInt64) -> UInt64
```

`self - other`, wrapping on overflow.

_Defined in `lang/std/num/uint64.ks`._

### Implements `Multipliable`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/arithmetic.ks`._

#### function `multiply`

```kestrel
public func multiply(UInt64) -> UInt64
```

`self * other`, wrapping on overflow.

_Defined in `lang/std/num/uint64.ks`._

#### field `one`

```kestrel
public static var one: UInt64 { get }
```

The multiplicative identity, `1`.

_Defined in `lang/std/num/uint64.ks`._

### Implements `Divisible`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/arithmetic.ks`._

#### function `divide`

```kestrel
public func divide(UInt64) -> UInt64
```

Truncating integer division (`self / other`). For signed types,
`minValue / -1` wraps; use `divideChecked` to detect.

##### Errors

Traps on division by zero (LLVM `udiv`/`sdiv` are UB on zero — the
process aborts before producing a result).

_Defined in `lang/std/num/uint64.ks`._

### Implements `Modulo`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/arithmetic.ks`._

#### function `modulo`

```kestrel
public func modulo(UInt64) -> UInt64
```

`self % other` — truncated remainder; the result has the sign of
`self` for signed types.

##### Errors

Traps on division by zero, like `divide`.

_Defined in `lang/std/num/uint64.ks`._

### Implements `BitwiseAnd`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/bitwise.ks`._

#### function `bitwiseAnd`

```kestrel
public func bitwiseAnd(UInt64) -> UInt64
```

Bitwise AND. `0b1010 & 0b1100 == 0b1000`.

_Defined in `lang/std/num/uint64.ks`._

### Implements `BitwiseOr`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/bitwise.ks`._

#### function `bitwiseOr`

```kestrel
public func bitwiseOr(UInt64) -> UInt64
```

Bitwise OR. `0b1010 | 0b1100 == 0b1110`.

_Defined in `lang/std/num/uint64.ks`._

### Implements `BitwiseXor`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/bitwise.ks`._

#### function `bitwiseXor`

```kestrel
public func bitwiseXor(UInt64) -> UInt64
```

Bitwise XOR. `0b1010 ^ 0b1100 == 0b0110`.

_Defined in `lang/std/num/uint64.ks`._

### Implements `BitwiseNot`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/bitwise.ks`._

#### function `bitwiseNot`

```kestrel
public func bitwiseNot() -> UInt64
```

Bitwise NOT — flips all bits. For signed types this is `-self - 1`.

_Defined in `lang/std/num/uint64.ks`._

### Implements `LeftShift`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/bitwise.ks`._

#### function `shiftLeft`

```kestrel
public func shiftLeft(by: lang.i64) -> UInt64
```

Left shift by `count`. Behavior is undefined when `count >= bitWidth`
— pre-mask the count if you can't guarantee the bound.

_Defined in `lang/std/num/uint64.ks`._

### Implements `RightShift`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/bitwise.ks`._

#### function `shiftRight`

```kestrel
public func shiftRight(by: lang.i64) -> UInt64
```

Right shift by `count`. Arithmetic (sign-extending) for signed types,
logical (zero-filling) for unsigned. Same `count` precondition as
`shiftLeft`.

_Defined in `lang/std/num/uint64.ks`._

### Implements `AddAssign`

#### function `addAssign`

```kestrel
public mutating func addAssign(UInt64)
```

`self += other`

_Defined in `lang/std/num/uint64.ks`._

### Implements `SubtractAssign`

#### function `subtractAssign`

```kestrel
public mutating func subtractAssign(UInt64)
```

`self -= other`

_Defined in `lang/std/num/uint64.ks`._

### Implements `MultiplyAssign`

#### function `multiplyAssign`

```kestrel
public mutating func multiplyAssign(UInt64)
```

`self *= other`

_Defined in `lang/std/num/uint64.ks`._

### Implements `DivideAssign`

#### function `divideAssign`

```kestrel
public mutating func divideAssign(UInt64)
```

`self /= other`

_Defined in `lang/std/num/uint64.ks`._

### Implements `ModuloAssign`

#### function `modAssign`

```kestrel
public mutating func modAssign(UInt64)
```

`self %= other`

_Defined in `lang/std/num/uint64.ks`._

### Implements `BitwiseAndAssign`

#### function `bitwiseAndAssign`

```kestrel
public mutating func bitwiseAndAssign(UInt64)
```

`self &= other`

_Defined in `lang/std/num/uint64.ks`._

### Implements `BitwiseOrAssign`

#### function `bitwiseOrAssign`

```kestrel
public mutating func bitwiseOrAssign(UInt64)
```

`self |= other`

_Defined in `lang/std/num/uint64.ks`._

### Implements `BitwiseXorAssign`

#### function `bitwiseXorAssign`

```kestrel
public mutating func bitwiseXorAssign(UInt64)
```

`self ^= other`

_Defined in `lang/std/num/uint64.ks`._

### Implements `LeftShiftAssign`

#### function `shiftLeftAssign`

```kestrel
public mutating func shiftLeftAssign(by: lang.i64)
```

`self <<= count`

_Defined in `lang/std/num/uint64.ks`._

### Implements `RightShiftAssign`

#### function `shiftRightAssign`

```kestrel
public mutating func shiftRightAssign(by: lang.i64)
```

`self >>= count`

_Defined in `lang/std/num/uint64.ks`._

### Implements `ExpressibleByIntLiteral`

#### initializer `Int Literal`

```kestrel
init(intLiteral: lang.i64)
```

Builds an instance from an integer literal.

_Defined in `lang/std/core/literals.ks`._

### Implements `Defaultable`

#### initializer `Default`

```kestrel
init()
```

Builds the default-valued instance.

_Defined in `lang/std/core/protocols.ks`._

### Implements `RangeConstructible`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/range.ks`._

#### function `exclusiveRange`

```kestrel
public func exclusiveRange(to: UInt64) -> Range[UInt64]
```

Builds a half-open range `self..<end`. Sugar for the `..<` operator.

_Defined in `lang/std/num/uint64.ks`._

### Implements `ClosedRangeConstructible`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/range.ks`._

#### function `inclusiveRange`

```kestrel
public func inclusiveRange(to: UInt64) -> ClosedRange[UInt64]
```

Builds a closed range `self..=end`. Sugar for the `..=` operator.

_Defined in `lang/std/num/uint64.ks`._

### Implements `Convertible`

#### initializer `From Source`

```kestrel
init(from: From)
```

Creates an instance from `value`.

_Defined in `lang/std/core/convertible.ks`._

## struct `UInt8`

```kestrel
public struct UInt8 { /* private fields */ }
```

A 8-bit unsigned integer.

UInt8 is the 8-bit member of the integer family. The same surface
area is provided across all widths; switch widths to trade range for memory
or to match an FFI ABI. Arithmetic wraps on overflow by default — use the
`*Checked` variants for overflow detection or `*Saturating` to clamp to
`minValue`/`maxValue`. The type is `FFISafe` and lays out as a single
`lang.i8` so it can cross C boundaries unchanged.

### Examples

```
let a: Int64 = 100;
let b = a + 50;        // 150
let c = a * 2;         // 200
let d = a.addChecked(Int64.maxValue);  // None (overflow detected)
```

```
// Bit twiddling
(0b1010).countOnes      // 2
(1).shiftLeft(by: 4)    // 16
(-1).leadingZeros       // 0  (all bits set)
```

### Representation

A single `lang.i8` field. No padding, no headers — bit-identical
to the corresponding C type.

_Defined in `lang/std/num/uint8.ks`._

### Members

#### initializer `Default`

```kestrel
public init()
```

Creates the zero value, satisfying `Defaultable`.

##### Examples

```
let n = Int64();   // 0
```

_Defined in `lang/std/num/uint8.ks`._

#### initializer `From Integer`

```kestrel
public init(from: Int8)
```

Converts from `Int8`. Narrowing conversions truncate the high
bits; signed→unsigned reinterprets the bit pattern.

_Defined in `lang/std/num/uint8.ks`._

#### initializer `From Integer`

```kestrel
public init(from: Int16)
```

Converts from `Int16`. Narrowing conversions truncate the high
bits; signed→unsigned reinterprets the bit pattern.

_Defined in `lang/std/num/uint8.ks`._

#### initializer `From Integer`

```kestrel
public init(from: Int32)
```

Converts from `Int32`. Narrowing conversions truncate the high
bits; signed→unsigned reinterprets the bit pattern.

_Defined in `lang/std/num/uint8.ks`._

#### initializer `From Integer`

```kestrel
public init(from: Int64)
```

Converts from `Int64`. Narrowing conversions truncate the high
bits; signed→unsigned reinterprets the bit pattern.

_Defined in `lang/std/num/uint8.ks`._

#### initializer `From Integer`

```kestrel
public init(from: UInt16)
```

Converts from `UInt16`. Narrowing conversions truncate the high
bits; signed→unsigned reinterprets the bit pattern.

_Defined in `lang/std/num/uint8.ks`._

#### initializer `From Integer`

```kestrel
public init(from: UInt32)
```

Converts from `UInt32`. Narrowing conversions truncate the high
bits; signed→unsigned reinterprets the bit pattern.

_Defined in `lang/std/num/uint8.ks`._

#### initializer `From Integer`

```kestrel
public init(from: UInt64)
```

Converts from `UInt64`. Narrowing conversions truncate the high
bits; signed→unsigned reinterprets the bit pattern.

_Defined in `lang/std/num/uint8.ks`._

#### initializer `From Raw`

```kestrel
init(raw: lang.i8)
```

Wraps an existing `lang.i8` without conversion. Internal
constructor used by intrinsics; not part of the public API.

_Defined in `lang/std/num/uint8.ks`._

#### initializer `Int Literal`

```kestrel
public init(intLiteral: lang.i64)
```

Compiler-emitted bridge that turns an integer literal into a UInt8.

You will rarely call this directly — write the literal and let the
`ExpressibleByIntLiteral` protocol pick it up. For widths smaller than
64 bits the literal is truncated with `lang.cast_i64_i8`.

##### Examples

```
let n: Int64 = 42;            // implicit
let m = Int64(intLiteral: 42);  // explicit
```

_Defined in `lang/std/num/uint8.ks`._

#### function `addChecked`

```kestrel
public func addChecked(UInt8) -> UInt8?
```

Wrapping addition that returns `None` on overflow. For unsigned types
overflow is detected via `result < self`.

_Defined in `lang/std/num/uint8.ks`._

#### function `addSaturating`

```kestrel
public func addSaturating(UInt8) -> UInt8
```

Addition that clamps to `maxValue` on overflow.

_Defined in `lang/std/num/uint8.ks`._

#### field `bitWidth`

```kestrel
public static var bitWidth: Int64 { get }
```

The width in bits (8). Useful for shift bounds and bit-walks.

_Defined in `lang/std/num/uint8.ks`._

#### field `byteSwapped`

```kestrel
public var byteSwapped: UInt8 { get }
```

Value with its byte order reversed. Use to convert between big- and
little-endian; lowered to a `bswap` intrinsic.

_Defined in `lang/std/num/uint8.ks`._

#### function `clamp`

```kestrel
public func clamp(UInt8, UInt8) -> UInt8
```

Clamps `self` into `[min, max]`. Caller is responsible for ensuring
`min <= max`; otherwise the result is undefined.

##### Examples

```
(5).clamp(min: 0, max: 10);    // 5
(-5).clamp(min: 0, max: 10);   // 0
(15).clamp(min: 0, max: 10);   // 10
```

_Defined in `lang/std/num/uint8.ks`._

#### field `countOnes`

```kestrel
public var countOnes: Int64 { get }
```

Population count — the number of `1` bits in the binary representation.

Lowered to a `popcount` intrinsic where the target supports it.

##### Examples

```
(0b1010).countOnes;  // 2
(0b1111).countOnes;  // 4
(0).countOnes;       // 0
```

_Defined in `lang/std/num/uint8.ks`._

#### field `countZeros`

```kestrel
public var countZeros: Int64 { get }
```

Complement of `countOnes`: equal to `bitWidth - countOnes`.

_Defined in `lang/std/num/uint8.ks`._

#### function `divideChecked`

```kestrel
public func divideChecked(UInt8) -> UInt8?
```

Division that returns `None` for divide-by-zero.

_Defined in `lang/std/num/uint8.ks`._

#### function `fromBytes`

```kestrel
public static func fromBytes(std.collections.Array[UInt8]) -> UInt8?
```

Reassembles a `UInt8` from 1 bytes in native (host) byte
order. Returns `None` if the input is not exactly 1 bytes long.

_Defined in `lang/std/num/uint8.ks`._

#### function `fromBytesBigEndian`

```kestrel
public static func fromBytesBigEndian(std.collections.Array[UInt8]) -> UInt8?
```

Reassembles a `UInt8` from 1 bytes in big-endian order.
Returns `None` if the input is not exactly 1 bytes long.

_Defined in `lang/std/num/uint8.ks`._

#### function `fromBytesLittleEndian`

```kestrel
public static func fromBytesLittleEndian(std.collections.Array[UInt8]) -> UInt8?
```

Reassembles a `UInt8` from 1 bytes in little-endian order.
Returns `None` if the input is not exactly 1 bytes long.

_Defined in `lang/std/num/uint8.ks`._

#### function `gcd`

```kestrel
public func gcd(UInt8) -> UInt8
```

Greatest common divisor via Euclidean algorithm. For signed types
the inputs are taken absolute first; the result is always non-negative.

##### Examples

```
(12).gcd(8);   // 4
(17).gcd(5);   // 1   (coprime)
(-12).gcd(8);  // 4
```

_Defined in `lang/std/num/uint8.ks`._

#### field `isNegative`

```kestrel
public var isNegative: Bool { get }
```

Always `false` — unsigned types cannot be negative.

_Defined in `lang/std/num/uint8.ks`._

#### field `isPositive`

```kestrel
public var isPositive: Bool { get }
```

True when `self > 0`.

_Defined in `lang/std/num/uint8.ks`._

#### field `isPowerOfTwo`

```kestrel
public var isPowerOfTwo: Bool { get }
```

True when the value is a positive power of two (`2^k` for `k >= 0`).

Zero and negatives are excluded. Cheap branchless test built on
`x & (x - 1) == 0`.

##### Examples

```
(1).isPowerOfTwo;   // true  (2^0)
(4).isPowerOfTwo;   // true  (2^2)
(3).isPowerOfTwo;   // false
(0).isPowerOfTwo;   // false
```

_Defined in `lang/std/num/uint8.ks`._

#### field `isZero`

```kestrel
public var isZero: Bool { get }
```

True when `self == 0`.

_Defined in `lang/std/num/uint8.ks`._

#### function `lcm`

```kestrel
public func lcm(UInt8) -> UInt8
```

Least common multiple, computed as `|self| / gcd(self, other) * |other|`
to avoid intermediate overflow. Returns zero if either input is zero.

##### Examples

```
(4).lcm(6);   // 12
(3).lcm(5);   // 15
(0).lcm(7);   // 0
```

_Defined in `lang/std/num/uint8.ks`._

#### field `leadingZeros`

```kestrel
public var leadingZeros: Int64 { get }
```

Number of leading zero bits, counting from the most-significant end.

For zero, returns `bitWidth`.

##### Examples

```
(1).leadingZeros;   // bitWidth - 1
(0).leadingZeros;   // bitWidth
```

_Defined in `lang/std/num/uint8.ks`._

#### field `maxValue`

```kestrel
public static var maxValue: UInt8 { get }
```

The largest representable value.
This is 2^8 - 1 (255).

_Defined in `lang/std/num/uint8.ks`._

#### field `minValue`

```kestrel
public static var minValue: UInt8 { get }
```

The smallest representable value.
This is always 0 for unsigned types.
Note that for signed types `minValue.negate()` overflows back to
itself; use `negateChecked()` if you need to detect that.

_Defined in `lang/std/num/uint8.ks`._

#### function `multiplyChecked`

```kestrel
public func multiplyChecked(UInt8) -> UInt8?
```

Wrapping multiplication that returns `None` on overflow. Implemented
by multiplying then dividing back.

_Defined in `lang/std/num/uint8.ks`._

#### function `multiplySaturating`

```kestrel
public func multiplySaturating(UInt8) -> UInt8
```

Multiplication that clamps to `maxValue` on overflow.

_Defined in `lang/std/num/uint8.ks`._

#### function `parse`

```kestrel
public static func parse(String) -> UInt8?
```

Parses a base-10 unsigned integer literal, optionally prefixed
with `+`. A leading `-` is rejected. Returns `None` for an empty
string, a non-digit character, or a value that does not fit in
`UInt8`.

##### Examples

```
UInt8.parse(string: "42");   // Some(42)
UInt8.parse(string: "-1");   // None  (no sign for unsigned)
UInt8.parse(string: "");     // None
```

_Defined in `lang/std/num/uint8.ks`._

#### function `parse`

```kestrel
public static func parse(String, Int64) -> UInt8?
```

Parses an unsigned integer in `radix` (base 2–36 inclusive). Letters
a–z are case-insensitive and represent digit values 10–35. A
leading `+` is allowed but a leading `-` is rejected. Returns
`None` for an out-of-range radix, an empty string, an
unrecognised digit, or a value that overflows `UInt8`.

##### Examples

```
UInt8.parse(string: "ff", radix: 16);     // Some(255 if it fits, else None)
UInt8.parse(string: "101010", radix: 2);  // Some(42)
```

_Defined in `lang/std/num/uint8.ks`._

#### function `pow`

```kestrel
public func pow(Int64) -> UInt8
```

Raises `self` to `exponent` via binary exponentiation. Wraps on
overflow. Negative exponents return zero (integer truncation of
the would-be fraction).

##### Examples

```
(2).pow(10);  // 1024
(3).pow(4);   // 81
(5).pow(-1);  // 0
```

_Defined in `lang/std/num/uint8.ks`._

#### field `raw`

```kestrel
public var raw: lang.i8
```

The underlying primitive `lang.i8` value. Exposed for FFI
and intrinsic use; prefer the typed surface for everything else.

_Defined in `lang/std/num/uint8.ks`._

#### function `rotateLeft`

```kestrel
public func rotateLeft(by: Int64) -> UInt8
```

Rotates bits left by `count`, modulo `bitWidth`. Bits shifted past the
MSB re-enter at the LSB.

_Defined in `lang/std/num/uint8.ks`._

#### function `rotateRight`

```kestrel
public func rotateRight(by: Int64) -> UInt8
```

Rotates bits right by `count`, modulo `bitWidth`. Mirror of
`rotateLeft`.

_Defined in `lang/std/num/uint8.ks`._

#### field `sign`

```kestrel
public var sign: UInt8 { get }
```

Sign as a `UInt8`: `0` for zero, `1` otherwise (unsigned types
have no negative values).

_Defined in `lang/std/num/uint8.ks`._

#### function `subtractChecked`

```kestrel
public func subtractChecked(UInt8) -> UInt8?
```

Subtraction that returns `None` on underflow (`other > self`).

_Defined in `lang/std/num/uint8.ks`._

#### function `subtractSaturating`

```kestrel
public func subtractSaturating(UInt8) -> UInt8
```

Subtraction that clamps to `0` on underflow (unsigned types cannot
represent negative results).

_Defined in `lang/std/num/uint8.ks`._

#### function `toBytes`

```kestrel
public func toBytes() -> std.collections.Array[UInt8]
```

Splits this integer into 1 bytes in *native* (host) byte order.
Use `toBytesBigEndian` / `toBytesLittleEndian` when serialising for
a fixed wire format.

##### Examples

```
let bytes = UInt8.maxValue.toBytes();   // 1 bytes, host order
```

_Defined in `lang/std/num/uint8.ks`._

#### function `toBytesBigEndian`

```kestrel
public func toBytesBigEndian() -> std.collections.Array[UInt8]
```

Splits this integer into 1 bytes in big-endian order (most
significant byte first — i.e. network byte order).

_Defined in `lang/std/num/uint8.ks`._

#### function `toBytesLittleEndian`

```kestrel
public func toBytesLittleEndian() -> std.collections.Array[UInt8]
```

Splits this integer into 1 bytes in little-endian order (least
significant byte first).

_Defined in `lang/std/num/uint8.ks`._

#### field `trailingZeros`

```kestrel
public var trailingZeros: Int64 { get }
```

Number of trailing zero bits. Equal to `log2(self & -self)` for non-zero
values; returns `bitWidth` for zero. Useful for finding the largest
power of two dividing the value.

_Defined in `lang/std/num/uint8.ks`._

### Implements `Steppable`

#### function `predecessor`

```kestrel
public func predecessor() -> UInt8
```

Predecessor — `self - 1`. Wraps at `minValue`.

_Defined in `lang/std/num/uint8.ks`._

#### function `successor`

```kestrel
public func successor() -> UInt8
```

Successor — `self + 1`. Wraps at `maxValue`. Used by `for-in` over
integer ranges.

_Defined in `lang/std/num/uint8.ks`._

### Implements `Comparable`

#### function `compare`

```kestrel
public func compare(UInt8) -> Ordering
```

Three-way comparison returning an `Ordering`. Signed types compare
using two's-complement ordering; unsigned types use natural ordering.

##### Examples

```
(1).compare(other: 2);   // .Less
(2).compare(other: 2);   // .Equal
(3).compare(other: 2);   // .Greater
```

_Defined in `lang/std/num/uint8.ks`._

### Implements `Equatable`

#### function `equals`

```kestrel
public func equals(UInt8) -> Bool
```

Bit-for-bit equality. Backs the `==` operator.

##### Examples

```
(42).equals(other: 42);  // true
42 == 42;                // true
```

_Defined in `lang/std/num/uint8.ks`._

### Implements `Matchable`

#### function `matches`

```kestrel
public func matches(UInt8) -> Bool
```

Pattern-matching hook for `Matchable`. Identical to `equals`.

_Defined in `lang/std/num/uint8.ks`._

### Implements `Formattable`

#### function `format`

```kestrel
public func format(FormatOptions) -> String
```

Renders the integer to a `String`, honouring the supplied
`FormatOptions`. Implements the `Formattable` protocol.

Recognised options:
- `radix` — base in `[2, 36]`; out-of-range values fall back to 10.
- `width` — minimum output width; shorter values are padded.
- `fill` / `alignment` — padding character and side.
- `sign` — `.Negative` (default), `.Always`, or `.Space`.
- `uppercase` — uppercase hex digits.
- `alternate` — emit the `0b` / `0o` / `0x` prefix.

##### Examples

```
(42).format();                                           // "42"
(255).format(options: .{radix: 16});                     // "ff"
(255).format(options: .{radix: 16, uppercase: true});    // "FF"
(255).format(options: .{radix: 16, alternate: true});    // "0xff"
(42).format(options: .{radix: 2, alternate: true});      // "0b101010"
(42).format(options: .{width: .Some(5), fill: '0'});     // "00042"
(-42).format(options: .{sign: .Always});                 // "-42"
```

_Defined in `lang/std/num/uint8.ks`._

### Implements `Hash`

#### function `hash`

```kestrel
public func hash[H](into: mutating H) where H: Hasher
```

Feeds the raw bytes of this value into `hasher`. Endianness-agnostic
only within a single process — do not persist hashes across builds.

_Defined in `lang/std/num/uint8.ks`._

### Implements `Addable`

#### typealias `Output`

```kestrel
type Output = UInt8
```

_Defined in `lang/std/num/uint8.ks`._

#### typealias `Output`

```kestrel
type Output = UInt8
```

_Defined in `lang/std/num/uint8.ks`._

#### typealias `Output`

```kestrel
type Output = UInt8
```

_Defined in `lang/std/num/uint8.ks`._

#### typealias `Output`

```kestrel
type Output = UInt8
```

_Defined in `lang/std/num/uint8.ks`._

#### typealias `Output`

```kestrel
type Output = UInt8
```

_Defined in `lang/std/num/uint8.ks`._

#### typealias `Output`

```kestrel
type Output = UInt8
```

_Defined in `lang/std/num/uint8.ks`._

#### typealias `Output`

```kestrel
type Output = UInt8
```

_Defined in `lang/std/num/uint8.ks`._

#### typealias `Output`

```kestrel
type Output = UInt8
```

_Defined in `lang/std/num/uint8.ks`._

#### typealias `Output`

```kestrel
type Output = UInt8
```

_Defined in `lang/std/num/uint8.ks`._

#### typealias `Output`

```kestrel
type Output = UInt8
```

_Defined in `lang/std/num/uint8.ks`._

#### typealias `Output`

```kestrel
type Output = UInt8
```

_Defined in `lang/std/num/uint8.ks`._

#### typealias `Output`

```kestrel
type Output = Range[UInt8]
```

_Defined in `lang/std/num/uint8.ks`._

#### typealias `Output`

```kestrel
type Output = ClosedRange[UInt8]
```

_Defined in `lang/std/num/uint8.ks`._

#### function `add`

```kestrel
public func add(UInt8) -> UInt8
```

`self + other`, wrapping on overflow. Use `addChecked` to detect or
`addSaturating` to clamp.

_Defined in `lang/std/num/uint8.ks`._

#### field `zero`

```kestrel
public static var zero: UInt8 { get }
```

The additive identity, `0`.

_Defined in `lang/std/num/uint8.ks`._

### Implements `Subtractable`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/arithmetic.ks`._

#### function `subtract`

```kestrel
public func subtract(UInt8) -> UInt8
```

`self - other`, wrapping on overflow.

_Defined in `lang/std/num/uint8.ks`._

### Implements `Multipliable`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/arithmetic.ks`._

#### function `multiply`

```kestrel
public func multiply(UInt8) -> UInt8
```

`self * other`, wrapping on overflow.

_Defined in `lang/std/num/uint8.ks`._

#### field `one`

```kestrel
public static var one: UInt8 { get }
```

The multiplicative identity, `1`.

_Defined in `lang/std/num/uint8.ks`._

### Implements `Divisible`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/arithmetic.ks`._

#### function `divide`

```kestrel
public func divide(UInt8) -> UInt8
```

Truncating integer division (`self / other`). For signed types,
`minValue / -1` wraps; use `divideChecked` to detect.

##### Errors

Traps on division by zero (LLVM `udiv`/`sdiv` are UB on zero — the
process aborts before producing a result).

_Defined in `lang/std/num/uint8.ks`._

### Implements `Modulo`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/arithmetic.ks`._

#### function `modulo`

```kestrel
public func modulo(UInt8) -> UInt8
```

`self % other` — truncated remainder; the result has the sign of
`self` for signed types.

##### Errors

Traps on division by zero, like `divide`.

_Defined in `lang/std/num/uint8.ks`._

### Implements `BitwiseAnd`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/bitwise.ks`._

#### function `bitwiseAnd`

```kestrel
public func bitwiseAnd(UInt8) -> UInt8
```

Bitwise AND. `0b1010 & 0b1100 == 0b1000`.

_Defined in `lang/std/num/uint8.ks`._

### Implements `BitwiseOr`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/bitwise.ks`._

#### function `bitwiseOr`

```kestrel
public func bitwiseOr(UInt8) -> UInt8
```

Bitwise OR. `0b1010 | 0b1100 == 0b1110`.

_Defined in `lang/std/num/uint8.ks`._

### Implements `BitwiseXor`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/bitwise.ks`._

#### function `bitwiseXor`

```kestrel
public func bitwiseXor(UInt8) -> UInt8
```

Bitwise XOR. `0b1010 ^ 0b1100 == 0b0110`.

_Defined in `lang/std/num/uint8.ks`._

### Implements `BitwiseNot`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/bitwise.ks`._

#### function `bitwiseNot`

```kestrel
public func bitwiseNot() -> UInt8
```

Bitwise NOT — flips all bits. For signed types this is `-self - 1`.

_Defined in `lang/std/num/uint8.ks`._

### Implements `LeftShift`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/bitwise.ks`._

#### function `shiftLeft`

```kestrel
public func shiftLeft(by: lang.i64) -> UInt8
```

Left shift by `count`. Behavior is undefined when `count >= bitWidth`
— pre-mask the count if you can't guarantee the bound.

_Defined in `lang/std/num/uint8.ks`._

### Implements `RightShift`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/bitwise.ks`._

#### function `shiftRight`

```kestrel
public func shiftRight(by: lang.i64) -> UInt8
```

Right shift by `count`. Arithmetic (sign-extending) for signed types,
logical (zero-filling) for unsigned. Same `count` precondition as
`shiftLeft`.

_Defined in `lang/std/num/uint8.ks`._

### Implements `AddAssign`

#### function `addAssign`

```kestrel
public mutating func addAssign(UInt8)
```

`self += other`

_Defined in `lang/std/num/uint8.ks`._

### Implements `SubtractAssign`

#### function `subtractAssign`

```kestrel
public mutating func subtractAssign(UInt8)
```

`self -= other`

_Defined in `lang/std/num/uint8.ks`._

### Implements `MultiplyAssign`

#### function `multiplyAssign`

```kestrel
public mutating func multiplyAssign(UInt8)
```

`self *= other`

_Defined in `lang/std/num/uint8.ks`._

### Implements `DivideAssign`

#### function `divideAssign`

```kestrel
public mutating func divideAssign(UInt8)
```

`self /= other`

_Defined in `lang/std/num/uint8.ks`._

### Implements `ModuloAssign`

#### function `modAssign`

```kestrel
public mutating func modAssign(UInt8)
```

`self %= other`

_Defined in `lang/std/num/uint8.ks`._

### Implements `BitwiseAndAssign`

#### function `bitwiseAndAssign`

```kestrel
public mutating func bitwiseAndAssign(UInt8)
```

`self &= other`

_Defined in `lang/std/num/uint8.ks`._

### Implements `BitwiseOrAssign`

#### function `bitwiseOrAssign`

```kestrel
public mutating func bitwiseOrAssign(UInt8)
```

`self |= other`

_Defined in `lang/std/num/uint8.ks`._

### Implements `BitwiseXorAssign`

#### function `bitwiseXorAssign`

```kestrel
public mutating func bitwiseXorAssign(UInt8)
```

`self ^= other`

_Defined in `lang/std/num/uint8.ks`._

### Implements `LeftShiftAssign`

#### function `shiftLeftAssign`

```kestrel
public mutating func shiftLeftAssign(by: lang.i64)
```

`self <<= count`

_Defined in `lang/std/num/uint8.ks`._

### Implements `RightShiftAssign`

#### function `shiftRightAssign`

```kestrel
public mutating func shiftRightAssign(by: lang.i64)
```

`self >>= count`

_Defined in `lang/std/num/uint8.ks`._

### Implements `ExpressibleByIntLiteral`

#### initializer `Int Literal`

```kestrel
init(intLiteral: lang.i64)
```

Builds an instance from an integer literal.

_Defined in `lang/std/core/literals.ks`._

### Implements `Defaultable`

#### initializer `Default`

```kestrel
init()
```

Builds the default-valued instance.

_Defined in `lang/std/core/protocols.ks`._

### Implements `RangeConstructible`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/range.ks`._

#### function `exclusiveRange`

```kestrel
public func exclusiveRange(to: UInt8) -> Range[UInt8]
```

Builds a half-open range `self..<end`. Sugar for the `..<` operator.

_Defined in `lang/std/num/uint8.ks`._

### Implements `ClosedRangeConstructible`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/range.ks`._

#### function `inclusiveRange`

```kestrel
public func inclusiveRange(to: UInt8) -> ClosedRange[UInt8]
```

Builds a closed range `self..=end`. Sugar for the `..=` operator.

_Defined in `lang/std/num/uint8.ks`._

### Implements `Convertible`

#### initializer `From Source`

```kestrel
init(from: From)
```

Creates an instance from `value`.

_Defined in `lang/std/core/convertible.ks`._

## protocol `UnsignedInteger`

```kestrel
public protocol UnsignedInteger
```

Marker protocol for unsigned integer types. Carries no requirements —
it exists so generic code can constrain on signedness without naming
every concrete `UInt*` type.

_Defined in `lang/std/num/numeric.ks`._

