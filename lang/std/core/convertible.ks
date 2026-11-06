// Type conversion protocol

module std.core

/// Protocol for explicit type conversions via `init(from:)`.
///
/// Conform when you want callers to write `Target(from: source)`. Most
/// numeric types do this for every other numeric width (see
/// `lang/std/num/integer.ks.template`). Conformances should be lossless or
/// document their loss behavior; for fallible conversions prefer a separate
/// `Result`-returning function.
///
/// # Examples
///
/// ```
/// let i: Int64 = 42;
/// let u = UInt32(from: i);   // explicit narrowing conversion
/// ```
public protocol Convertible[From] {
    /// @name From Source
    /// Creates an instance from `value`.
    init(from value: From)
}

/// Protocol for the explicit lossy forms of a conversion, for sources
/// where `init(from:)` would trap because the value may not fit.
///
/// Each integer type conforms once per source it can't always hold, so
/// `Int8(truncating: x)` and `Int8(clamping: x)` work for every wider or
/// differently-signed `x`.
///
/// # Examples
///
/// ```
/// let big: Int64 = 300;
/// Int8(truncating: big);   // 44  (low 8 bits)
/// Int8(clamping: big);     // 127
/// ```
public protocol LossyConvertible[From] {
    /// @name Truncating
    /// Keeps the low bits of `value`, reinterpreted as `Self`. Never traps.
    init(truncating value: From)
    /// @name Clamping
    /// Converts `value`, saturating to `Self`'s minimum or maximum when it
    /// doesn't fit. Never traps.
    init(clamping value: From)
}
