// Numeric protocols

module std.numeric

/// A type whose values can be stepped one position at a time. Underpins
/// `for-in` over integer ranges and any other "next/previous" walk where
/// the step size is implicit (`1` for integers).
///
/// `successor` and `predecessor` should be inverses for every interior
/// value; behaviour at the type's edges (`Int64.maxValue.successor()`,
/// for example) follows the same wrapping rules as `add`/`subtract`.
public protocol Steppable {
    /// The next value in the sequence. For integers this is `self + 1`.
    func successor() -> Self
    /// The previous value in the sequence. For integers this is `self - 1`.
    func predecessor() -> Self
}

/// Marker protocol for signed integer types. The `abs()` requirement is
/// what justifies treating these uniformly in generic code — unsigned
/// integers can't satisfy it without changing semantics.
public protocol SignedInteger {
    /// Absolute value. For two's-complement types this can overflow at
    /// `minValue`; consumers that need a total function should use
    /// `absChecked()` from the concrete type instead.
    func abs() -> Self
}

/// Marker protocol for unsigned integer types. Carries no requirements —
/// it exists so generic code can constrain on signedness without naming
/// every concrete `UInt*` type.
public protocol UnsignedInteger {}
