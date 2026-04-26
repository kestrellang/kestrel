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
