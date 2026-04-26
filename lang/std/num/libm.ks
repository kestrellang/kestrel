// libm bindings for mathematical functions
//
// Thin wrappers around C standard library math functions. Prefer the typed
// methods on `Float32` / `Float64` (which forward to these); call these
// directly only from low-level code that already has raw `lang.f32` /
// `lang.f64` values.

module std.num

// ============================================================================
// Float64 (double) functions
// ============================================================================

// Trigonometric

/// `sin(x)` — sine of `x` in radians.
@extern(.C, mangleName: "sin")
func libm_sin(x: lang.f64) -> lang.f64

/// `cos(x)` — cosine of `x` in radians.
@extern(.C, mangleName: "cos")
func libm_cos(x: lang.f64) -> lang.f64

/// `tan(x)` — tangent of `x` in radians.
@extern(.C, mangleName: "tan")
func libm_tan(x: lang.f64) -> lang.f64

/// `asin(x)` — arc sine, result in `[-π/2, π/2]`. NaN outside `[-1, 1]`.
@extern(.C, mangleName: "asin")
func libm_asin(x: lang.f64) -> lang.f64

/// `acos(x)` — arc cosine, result in `[0, π]`. NaN outside `[-1, 1]`.
@extern(.C, mangleName: "acos")
func libm_acos(x: lang.f64) -> lang.f64

/// `atan(x)` — arc tangent, result in `[-π/2, π/2]`.
@extern(.C, mangleName: "atan")
func libm_atan(x: lang.f64) -> lang.f64

/// `atan2(y, x)` — angle of point `(x, y)` from positive x-axis, in `[-π, π]`.
@extern(.C, mangleName: "atan2")
func libm_atan2(y: lang.f64, x: lang.f64) -> lang.f64

// Hyperbolic

/// `sinh(x)` — hyperbolic sine.
@extern(.C, mangleName: "sinh")
func libm_sinh(x: lang.f64) -> lang.f64

/// `cosh(x)` — hyperbolic cosine.
@extern(.C, mangleName: "cosh")
func libm_cosh(x: lang.f64) -> lang.f64

/// `tanh(x)` — hyperbolic tangent.
@extern(.C, mangleName: "tanh")
func libm_tanh(x: lang.f64) -> lang.f64

/// `asinh(x)` — inverse hyperbolic sine.
@extern(.C, mangleName: "asinh")
func libm_asinh(x: lang.f64) -> lang.f64

/// `acosh(x)` — inverse hyperbolic cosine. NaN for `x < 1`.
@extern(.C, mangleName: "acosh")
func libm_acosh(x: lang.f64) -> lang.f64

/// `atanh(x)` — inverse hyperbolic tangent. `±inf` at `±1`, NaN outside `(-1, 1)`.
@extern(.C, mangleName: "atanh")
func libm_atanh(x: lang.f64) -> lang.f64

// Exponential and Logarithmic

/// `exp(x)` — `e^x`.
@extern(.C, mangleName: "exp")
func libm_exp(x: lang.f64) -> lang.f64

/// `exp2(x)` — `2^x`.
@extern(.C, mangleName: "exp2")
func libm_exp2(x: lang.f64) -> lang.f64

/// `expm1(x)` — `e^x - 1`, accurate for small `x`.
@extern(.C, mangleName: "expm1")
func libm_expm1(x: lang.f64) -> lang.f64

/// `log(x)` — natural logarithm. NaN for negatives, `-inf` at zero.
@extern(.C, mangleName: "log")
func libm_log(x: lang.f64) -> lang.f64

/// `log2(x)` — base-2 logarithm.
@extern(.C, mangleName: "log2")
func libm_log2(x: lang.f64) -> lang.f64

/// `log10(x)` — base-10 logarithm.
@extern(.C, mangleName: "log10")
func libm_log10(x: lang.f64) -> lang.f64

/// `log1p(x)` — `ln(1 + x)`, accurate for small `x`.
@extern(.C, mangleName: "log1p")
func libm_log1p(x: lang.f64) -> lang.f64

// Power and Root

/// `pow(base, exp)` — `base^exp`. NaN for negative `base` with non-integer `exp`.
@extern(.C, mangleName: "pow")
func libm_pow(base: lang.f64, exp: lang.f64) -> lang.f64

/// `cbrt(x)` — cube root, defined for negatives.
@extern(.C, mangleName: "cbrt")
func libm_cbrt(x: lang.f64) -> lang.f64

/// `hypot(x, y)` — `sqrt(x² + y²)` without intermediate overflow.
@extern(.C, mangleName: "hypot")
func libm_hypot(x: lang.f64, y: lang.f64) -> lang.f64

// IEEE 754 Operations

/// `remainder(x, y)` — IEEE 754 remainder using round-to-nearest division.
@extern(.C, mangleName: "remainder")
func libm_remainder(x: lang.f64, y: lang.f64) -> lang.f64

/// `nextafter(x, y)` — next representable `f64` value from `x` toward `y`.
@extern(.C, mangleName: "nextafter")
func libm_nextafter(x: lang.f64, y: lang.f64) -> lang.f64

// ============================================================================
// Float32 (float) functions — mirror of the f64 set with `f` suffix.
// ============================================================================

// Trigonometric

/// `sinf(x)` — `f32` sine.
@extern(.C, mangleName: "sinf")
func libm_sinf(x: lang.f32) -> lang.f32

/// `cosf(x)` — `f32` cosine.
@extern(.C, mangleName: "cosf")
func libm_cosf(x: lang.f32) -> lang.f32

/// `tanf(x)` — `f32` tangent.
@extern(.C, mangleName: "tanf")
func libm_tanf(x: lang.f32) -> lang.f32

/// `asinf(x)` — `f32` arc sine.
@extern(.C, mangleName: "asinf")
func libm_asinf(x: lang.f32) -> lang.f32

/// `acosf(x)` — `f32` arc cosine.
@extern(.C, mangleName: "acosf")
func libm_acosf(x: lang.f32) -> lang.f32

/// `atanf(x)` — `f32` arc tangent.
@extern(.C, mangleName: "atanf")
func libm_atanf(x: lang.f32) -> lang.f32

/// `atan2f(y, x)` — `f32` two-argument arc tangent.
@extern(.C, mangleName: "atan2f")
func libm_atan2f(y: lang.f32, x: lang.f32) -> lang.f32

// Hyperbolic

/// `sinhf(x)` — `f32` hyperbolic sine.
@extern(.C, mangleName: "sinhf")
func libm_sinhf(x: lang.f32) -> lang.f32

/// `coshf(x)` — `f32` hyperbolic cosine.
@extern(.C, mangleName: "coshf")
func libm_coshf(x: lang.f32) -> lang.f32

/// `tanhf(x)` — `f32` hyperbolic tangent.
@extern(.C, mangleName: "tanhf")
func libm_tanhf(x: lang.f32) -> lang.f32

/// `asinhf(x)` — `f32` inverse hyperbolic sine.
@extern(.C, mangleName: "asinhf")
func libm_asinhf(x: lang.f32) -> lang.f32

/// `acoshf(x)` — `f32` inverse hyperbolic cosine.
@extern(.C, mangleName: "acoshf")
func libm_acoshf(x: lang.f32) -> lang.f32

/// `atanhf(x)` — `f32` inverse hyperbolic tangent.
@extern(.C, mangleName: "atanhf")
func libm_atanhf(x: lang.f32) -> lang.f32

// Exponential and Logarithmic

/// `expf(x)` — `f32` `e^x`.
@extern(.C, mangleName: "expf")
func libm_expf(x: lang.f32) -> lang.f32

/// `exp2f(x)` — `f32` `2^x`.
@extern(.C, mangleName: "exp2f")
func libm_exp2f(x: lang.f32) -> lang.f32

/// `expm1f(x)` — `f32` `e^x - 1`.
@extern(.C, mangleName: "expm1f")
func libm_expm1f(x: lang.f32) -> lang.f32

/// `logf(x)` — `f32` natural log.
@extern(.C, mangleName: "logf")
func libm_logf(x: lang.f32) -> lang.f32

/// `log2f(x)` — `f32` base-2 log.
@extern(.C, mangleName: "log2f")
func libm_log2f(x: lang.f32) -> lang.f32

/// `log10f(x)` — `f32` base-10 log.
@extern(.C, mangleName: "log10f")
func libm_log10f(x: lang.f32) -> lang.f32

/// `log1pf(x)` — `f32` `ln(1 + x)`.
@extern(.C, mangleName: "log1pf")
func libm_log1pf(x: lang.f32) -> lang.f32

// Power and Root

/// `powf(base, exp)` — `f32` power.
@extern(.C, mangleName: "powf")
func libm_powf(base: lang.f32, exp: lang.f32) -> lang.f32

/// `cbrtf(x)` — `f32` cube root.
@extern(.C, mangleName: "cbrtf")
func libm_cbrtf(x: lang.f32) -> lang.f32

/// `hypotf(x, y)` — `f32` hypotenuse.
@extern(.C, mangleName: "hypotf")
func libm_hypotf(x: lang.f32, y: lang.f32) -> lang.f32

// IEEE 754 Operations

/// `remainderf(x, y)` — `f32` IEEE 754 remainder.
@extern(.C, mangleName: "remainderf")
func libm_remainderf(x: lang.f32, y: lang.f32) -> lang.f32

/// `nextafterf(x, y)` — `f32` next representable value.
@extern(.C, mangleName: "nextafterf")
func libm_nextafterf(x: lang.f32, y: lang.f32) -> lang.f32
