// libm bindings for mathematical functions
//
// These are thin wrappers around C standard library math functions.
// Users should use Float32/Float64 methods rather than calling these directly.

module std.num

// ============================================================================
// Float64 (double) functions
// ============================================================================

// Trigonometric
@extern(.C, mangleName: "sin")
func libm_sin(x: lang.f64) -> lang.f64

@extern(.C, mangleName: "cos")
func libm_cos(x: lang.f64) -> lang.f64

@extern(.C, mangleName: "tan")
func libm_tan(x: lang.f64) -> lang.f64

@extern(.C, mangleName: "asin")
func libm_asin(x: lang.f64) -> lang.f64

@extern(.C, mangleName: "acos")
func libm_acos(x: lang.f64) -> lang.f64

@extern(.C, mangleName: "atan")
func libm_atan(x: lang.f64) -> lang.f64

@extern(.C, mangleName: "atan2")
func libm_atan2(y: lang.f64, x: lang.f64) -> lang.f64

// Hyperbolic
@extern(.C, mangleName: "sinh")
func libm_sinh(x: lang.f64) -> lang.f64

@extern(.C, mangleName: "cosh")
func libm_cosh(x: lang.f64) -> lang.f64

@extern(.C, mangleName: "tanh")
func libm_tanh(x: lang.f64) -> lang.f64

@extern(.C, mangleName: "asinh")
func libm_asinh(x: lang.f64) -> lang.f64

@extern(.C, mangleName: "acosh")
func libm_acosh(x: lang.f64) -> lang.f64

@extern(.C, mangleName: "atanh")
func libm_atanh(x: lang.f64) -> lang.f64

// Exponential and Logarithmic
@extern(.C, mangleName: "exp")
func libm_exp(x: lang.f64) -> lang.f64

@extern(.C, mangleName: "exp2")
func libm_exp2(x: lang.f64) -> lang.f64

@extern(.C, mangleName: "expm1")
func libm_expm1(x: lang.f64) -> lang.f64

@extern(.C, mangleName: "log")
func libm_log(x: lang.f64) -> lang.f64

@extern(.C, mangleName: "log2")
func libm_log2(x: lang.f64) -> lang.f64

@extern(.C, mangleName: "log10")
func libm_log10(x: lang.f64) -> lang.f64

@extern(.C, mangleName: "log1p")
func libm_log1p(x: lang.f64) -> lang.f64

// Power and Root
@extern(.C, mangleName: "pow")
func libm_pow(base: lang.f64, exp: lang.f64) -> lang.f64

@extern(.C, mangleName: "cbrt")
func libm_cbrt(x: lang.f64) -> lang.f64

@extern(.C, mangleName: "hypot")
func libm_hypot(x: lang.f64, y: lang.f64) -> lang.f64

// IEEE 754 Operations
@extern(.C, mangleName: "remainder")
func libm_remainder(x: lang.f64, y: lang.f64) -> lang.f64

@extern(.C, mangleName: "nextafter")
func libm_nextafter(x: lang.f64, y: lang.f64) -> lang.f64

// ============================================================================
// Float32 (float) functions - use 'f' suffix
// ============================================================================

// Trigonometric
@extern(.C, mangleName: "sinf")
func libm_sinf(x: lang.f32) -> lang.f32

@extern(.C, mangleName: "cosf")
func libm_cosf(x: lang.f32) -> lang.f32

@extern(.C, mangleName: "tanf")
func libm_tanf(x: lang.f32) -> lang.f32

@extern(.C, mangleName: "asinf")
func libm_asinf(x: lang.f32) -> lang.f32

@extern(.C, mangleName: "acosf")
func libm_acosf(x: lang.f32) -> lang.f32

@extern(.C, mangleName: "atanf")
func libm_atanf(x: lang.f32) -> lang.f32

@extern(.C, mangleName: "atan2f")
func libm_atan2f(y: lang.f32, x: lang.f32) -> lang.f32

// Hyperbolic
@extern(.C, mangleName: "sinhf")
func libm_sinhf(x: lang.f32) -> lang.f32

@extern(.C, mangleName: "coshf")
func libm_coshf(x: lang.f32) -> lang.f32

@extern(.C, mangleName: "tanhf")
func libm_tanhf(x: lang.f32) -> lang.f32

@extern(.C, mangleName: "asinhf")
func libm_asinhf(x: lang.f32) -> lang.f32

@extern(.C, mangleName: "acoshf")
func libm_acoshf(x: lang.f32) -> lang.f32

@extern(.C, mangleName: "atanhf")
func libm_atanhf(x: lang.f32) -> lang.f32

// Exponential and Logarithmic
@extern(.C, mangleName: "expf")
func libm_expf(x: lang.f32) -> lang.f32

@extern(.C, mangleName: "exp2f")
func libm_exp2f(x: lang.f32) -> lang.f32

@extern(.C, mangleName: "expm1f")
func libm_expm1f(x: lang.f32) -> lang.f32

@extern(.C, mangleName: "logf")
func libm_logf(x: lang.f32) -> lang.f32

@extern(.C, mangleName: "log2f")
func libm_log2f(x: lang.f32) -> lang.f32

@extern(.C, mangleName: "log10f")
func libm_log10f(x: lang.f32) -> lang.f32

@extern(.C, mangleName: "log1pf")
func libm_log1pf(x: lang.f32) -> lang.f32

// Power and Root
@extern(.C, mangleName: "powf")
func libm_powf(base: lang.f32, exp: lang.f32) -> lang.f32

@extern(.C, mangleName: "cbrtf")
func libm_cbrtf(x: lang.f32) -> lang.f32

@extern(.C, mangleName: "hypotf")
func libm_hypotf(x: lang.f32, y: lang.f32) -> lang.f32

// IEEE 754 Operations
@extern(.C, mangleName: "remainderf")
func libm_remainderf(x: lang.f32, y: lang.f32) -> lang.f32

@extern(.C, mangleName: "nextafterf")
func libm_nextafterf(x: lang.f32, y: lang.f32) -> lang.f32
