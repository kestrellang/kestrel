// Float32 - 32-bit floating point
// Generated from templates/float.ks.template

public struct Float32:
    FloatingPoint,
    Numeric,
    Addable,
    Subtractable,
    Multipliable,
    Divisible,
    Negatable,
    ExpressibleByFloatLiteral,
    ExpressibleByIntLiteral
{
    private var value: lang.f32

    public static var zero: Float32 { Float32(value: 0.0) }
    public static var one: Float32 { Float32(value: 1.0) }
    public static var infinity: Float32 { Float32(value: lang.f32_infinity()) }
    public static var nan: Float32 { Float32(value: lang.f32_nan()) }
    public static var bitWidth: Int { 32 }

    public init(floatLiteral value: Float64) {
        self.value = value as lang.f32
    }

    public init(intLiteral value: Int) {
        self.value = value as lang.f32
    }

    public func isNaN() -> Bool {
        lang.f32_is_nan(self.value)
    }

    public func isInfinite() -> Bool {
        lang.f32_is_infinite(self.value)
    }

    public func isFinite() -> Bool {
        not self.isNaN() and not self.isInfinite()
    }

    public func equals(other: Float32) -> Bool {
        lang.f32_eq(self.value, other.value)
    }

    public func compare(other: Float32) -> Ordering {
        if lang.f32_lt(self.value, other.value) { .Less }
        else if lang.f32_gt(self.value, other.value) { .Greater }
        else { .Equal }
    }

    type Output = Float32

    public func add(other: Float32) -> Float32 { Float32(value: lang.f32_add(self.value, other.value)) }
    public func subtract(other: Float32) -> Float32 { Float32(value: lang.f32_sub(self.value, other.value)) }
    public func multiply(other: Float32) -> Float32 { Float32(value: lang.f32_mul(self.value, other.value)) }
    public func divide(other: Float32) -> Float32 { Float32(value: lang.f32_div(self.value, other.value)) }
    public func negate() -> Float32 { Float32(value: lang.f32_neg(self.value)) }

    public func abs() -> Float32 {
        if lang.f32_lt(self.value, 0.0) { self.negate() } else { self }
    }

    public func floor() -> Float32 { Float32(value: lang.f32_floor(self.value)) }
    public func ceil() -> Float32 { Float32(value: lang.f32_ceil(self.value)) }
    public func round() -> Float32 { Float32(value: lang.f32_round(self.value)) }
    public func sqrt() -> Float32 { Float32(value: lang.f32_sqrt(self.value)) }
    public func pow(exponent: Float32) -> Float32 { Float32(value: lang.f32_pow(self.value, exponent.value)) }

    // Type conversions
    public func toInt() -> Int {
        Int64(value: self.value as lang.i64)
    }

    public func toInt8() -> Int8 {
        Int8(value: self.value as lang.i8)
    }

    public func toInt16() -> Int16 {
        Int16(value: self.value as lang.i16)
    }

    public func toInt32() -> Int32 {
        Int32(value: self.value as lang.i32)
    }

    public func toInt64() -> Int64 {
        Int64(value: self.value as lang.i64)
    }

    public func toFloat64() -> Float64 {
        Float64(value: self.value as lang.f64)
    }
}
