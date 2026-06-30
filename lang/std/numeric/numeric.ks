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
    /// The number of `successor()` steps from `self` to `other` (negative
    /// when `other` precedes `self`). For integers this is `other - self`.
    ///
    /// This is the `O(1)` stride distance — it lets range iterators carry a
    /// remaining-element *counter* instead of a boolean "finished" flag,
    /// which is what keeps `for x in a..=b` unrollable (a counter is an
    /// induction variable the optimizer can reason about; a flag is not).
    ///
    /// The result must fit in `Int64`. For spans wider than `Int64` (only
    /// reachable via near-full-width ranges, which never terminate in
    /// practice) the value wraps, following the same edge rule as
    /// `successor`/`predecessor`.
    func distance(to other: Self) -> Int64
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
