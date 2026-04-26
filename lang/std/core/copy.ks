// Copy semantics protocols

module std.core

/// Marker protocol for types whose values are duplicated by a plain bitwise
/// copy of their storage.
///
/// All built-in scalars and most plain value structs conform implicitly — the
/// compiler synthesises the conformance unless the type explicitly opts out
/// with `not Copyable`. Opt out for types that own a resource (a heap
/// allocation, a file handle) where bitwise duplication would alias the
/// resource and break ownership.
@builtin(.Copyable)
public protocol Copyable {}

/// Protocol for types that need custom logic when duplicated.
///
/// `Cloneable` extends `Copyable` so that cloneable values can flow through
/// generic code that asks only for `Copyable`. The compiler invokes
/// `clone()` automatically wherever a `Cloneable` value would otherwise be
/// implicitly copied (assignment, argument pass, return). The implementation
/// decides how deep the copy goes — `RcBox`, for example, only bumps the
/// refcount.
///
/// # Examples
///
/// ```
/// let a = RcBox(value: 1);
/// let b = a;            // implicit clone() — refcount bumps to 2
/// let c = a.clone();    // explicit clone — refcount bumps to 3
/// ```
@builtin(.Cloneable)
public protocol Cloneable: Copyable {
    /// Returns a copy of `self`. Conformers define the depth and any side
    /// effects (e.g. refcount adjustments).
    @builtin(.Clone)
    func clone() -> Self
}
