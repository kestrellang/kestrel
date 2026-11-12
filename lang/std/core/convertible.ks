// Type conversion protocol

module std.core

/// Protocol for explicit, lossless type conversions via `init(from:)`.
///
/// `Convertible[From]` promises that every `From` value converts without
/// loss and without trapping, so a `where Target: Convertible[T]` bound is
/// safe to call on any `T` it admits. Every type converts from itself (the
/// identity), so such a bound also accepts `Target`. Integers convert from
/// every narrower integer and floats from every integer their mantissa holds
/// exactly (see `lang/std/numeric/generate.py`). A source that doesn't always
/// fit is `NarrowsFrom` instead.
///
/// # Examples
///
/// ```
/// let i: Int32 = 42;
/// let w = Int64(from: i);       // widening: always fits
/// let big: Int64 = 300;
/// let n = Int8(exactly: big);   // nil: 300 doesn't fit
/// ```
public protocol Convertible[From] {
    /// @name From Source
    /// Creates an instance from `value`.
    init(from value: From)
}

/// Protocol for conversions from a source that doesn't always fit.
///
/// Each integer type conforms once per source it can't always hold, and
/// offers three spellings that each say what an out-of-range value does:
/// `exactly:` is `nil`, `truncating:` keeps the low bits, `clamping:`
/// saturates. Integers also narrow from `Float32` and `Float64`: there
/// `exactly:` is `nil` for a fraction, NaN or infinity too, and
/// `truncating:` and `clamping:` both drop the fraction toward zero,
/// saturate, and send NaN to 0.
///
/// # Examples
///
/// ```
/// let big: Int64 = 300;
/// Int8(exactly: big);      // nil
/// Int8(truncating: big);   // 44  (low 8 bits)
/// Int8(clamping: big);     // 127
/// Int64(truncating: -3.7); // -3
/// ```
public protocol NarrowsFrom[From] {
    /// @name Exactly
    /// Converts `value` when it fits in `Self`, and is `nil` when it doesn't.
    init(exactly value: From)?
    /// @name Truncating
    /// Keeps the low bits of `value`, reinterpreted as `Self`. Never traps.
    init(truncating value: From)
    /// @name Clamping
    /// Converts `value`, saturating to `Self`'s minimum or maximum when it
    /// doesn't fit. Never traps.
    init(clamping value: From)
}

/// Protocol for conversions that round to the nearest representable value.
///
/// A float conforms once per source whose values don't all fit its mantissa
/// (`Int64` into `Float64`, `Float64` into `Float32`), so the conversion's
/// name says it may lose low-order bits. A source that always converts
/// exactly is `Convertible` instead.
///
/// # Examples
///
/// ```
/// let n: Int64 = 9_007_199_254_740_993;   // 2^53 + 1
/// Float64(rounding: n);                   // 9007199254740992.0
/// ```
public protocol RoundsFrom[From] {
    /// @name Rounding
    /// Converts `value`, rounding to the nearest `Self`.
    init(rounding value: From)
}
