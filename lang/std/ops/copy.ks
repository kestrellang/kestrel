// Copy semantics protocols

module std.ops

/// Marker protocol for types that cannot be implicitly copied.
/// Types conforming to NonCopyable must be explicitly moved or cloned.
@builtin(.NonCopyable)
public protocol NonCopyable {}
