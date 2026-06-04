// ManuallyDrop[T] — wrapper that suppresses auto-drop on the inner value.
// Use for fields whose lifecycle is managed externally (e.g., heap-managed
// refcounted storage). The compiler never generates a destructor for this type.

module std.memory

@builtin(.ManuallyDropStruct)
public struct ManuallyDrop[T] {
    private var _value: T

    public init(value: T) {
        self._value = value
    }

    public var value: T {
        self._value
    }
}
