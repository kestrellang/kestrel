// Copy semantics protocols

module std.core

/// Marker protocol for types that can be implicitly copied (bitwise copy).
/// Types implicitly conform to Copyable unless opted out with `not Copyable`.
@builtin(.Copyable)
public protocol Copyable {}


/// Protocol for types that can be copied via a clone() method.
/// Unlike simple Copyable (bitwise copy), Cloneable types have custom copy behavior.
/// When a Cloneable value is copied, clone() is called automatically.
@builtin(.Cloneable)
public protocol Cloneable: Copyable {
    @builtin(.Clone)
    func clone() -> Self
}
