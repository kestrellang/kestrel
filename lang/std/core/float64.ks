// Float64 - 64-bit floating point
// Generated from templates/float.ks.template

import std.ffi.(FFISafe)

public struct Float64:
    FloatingPoint,
    Numeric,
    Addable,
    Subtractable,
    Multipliable,
    Divisible,
    Negatable,
    ExpressibleByFloatLiteral,
    ExpressibleByIntLiteral,
    FFISafe
{
    private var value: lang.f64

    public static var zero: Float64 { Float64(value: 0.0) }
    public static var one: Float64 { Float64(value: 1.0) }
    public static var infinity: Float64 { Float64(value: lang.f64_infinity()) }
    public static var nan: Float64 { Float64(value: lang.f64_nan()) }
    public static var bitWidth: Int { 64 }

    // Mathematical constants
    public static var pi: Float64 { Float64(value: 3.141592653589793) }
    public static var e: Float64 { Float64(value: 2.718281828459045) }
    public static var tau: Float64 { Float64(value: 6.283185307179586) }

    public init(floatLiteral value: Float64) {
        self.value = value.value
    }

    public init(intLiteral value: Int) {
        self.value = lang.cast_i64_f64(value)
    }

    public func isNaN() -> Bool {
        lang.f64_is_nan(self.value)
    }

    public func isInfinite() -> Bool {
        lang.f64_is_infinite(self.value)
    }

    public func isFinite() -> Bool {
        not self.isNaN() and not self.isInfinite()
    }

    public func equals(other: Float64) -> Bool {
        lang.f64_eq(self.value, other.value)
    }

    public func compare(other: Float64) -> Ordering {
        if lang.f64_lt(self.value, other.value) { .Less }
        else if lang.f64_gt(self.value, other.value) { .Greater }
        else { .Equal }
    }

    type Output = Float64

    public func add(other: Float64) -> Float64 { Float64(value: lang.f64_add(self.value, other.value)) }
    public func subtract(other: Float64) -> Float64 { Float64(value: lang.f64_sub(self.value, other.value)) }
    public func multiply(other: Float64) -> Float64 { Float64(value: lang.f64_mul(self.value, other.value)) }
    public func divide(other: Float64) -> Float64 { Float64(value: lang.f64_div(self.value, other.value)) }
    public func negate() -> Float64 { Float64(value: lang.f64_neg(self.value)) }

    public func abs() -> Float64 {
        if lang.f64_lt(self.value, 0.0) { self.negate() } else { self }
    }

    public func floor() -> Float64 { Float64(value: lang.f64_floor(self.value)) }
    public func ceil() -> Float64 { Float64(value: lang.f64_ceil(self.value)) }
    public func round() -> Float64 { Float64(value: lang.f64_round(self.value)) }
    public func trunc() -> Float64 { Float64(value: lang.f64_trunc(self.value)) }
    public func sqrt() -> Float64 { Float64(value: lang.f64_sqrt(self.value)) }
    public func pow(exponent: Float64) -> Float64 { Float64(value: lang.f64_pow(self.value, exponent.value)) }

    // Trigonometric functions
    public func sin() -> Float64 { Float64(value: lang.f64_sin(self.value)) }
    public func cos() -> Float64 { Float64(value: lang.f64_cos(self.value)) }
    public func tan() -> Float64 { Float64(value: lang.f64_tan(self.value)) }
    public func asin() -> Float64 { Float64(value: lang.f64_asin(self.value)) }
    public func acos() -> Float64 { Float64(value: lang.f64_acos(self.value)) }
    public func atan() -> Float64 { Float64(value: lang.f64_atan(self.value)) }
    public func atan2(x: Float64) -> Float64 { Float64(value: lang.f64_atan2(self.value, x.value)) }

    // Exponential and logarithmic
    public func exp() -> Float64 { Float64(value: lang.f64_exp(self.value)) }
    public func ln() -> Float64 { Float64(value: lang.f64_log(self.value)) }
    public func log10() -> Float64 { Float64(value: lang.f64_log10(self.value)) }
    public func log2() -> Float64 { Float64(value: lang.f64_log2(self.value)) }

    // Parsing
    public static func parse(string: String) -> Optional[Float64] {
        // Implementation would use lang intrinsics
        lang.f64_parse(string)
    }

    // Type conversions
    public func toInt() -> Int {
        Int64(value: lang.cast_f64_i64(self.value))
    }

    public func toInt8() -> Int8 {
        Int8(value: lang.cast_f64_i8(self.value))
    }

    public func toInt16() -> Int16 {
        Int16(value: lang.cast_f64_i16(self.value))
    }

    public func toInt32() -> Int32 {
        Int32(value: lang.cast_f64_i32(self.value))
    }

    public func toInt64() -> Int64 {
        Int64(value: lang.cast_f64_i64(self.value))
    }

    public func toFloat32() -> Float32 {
        Float32(value: lang.cast_f64_f32(self.value))
    }
}

// Float - alias to Float64
public type Float = Float64
