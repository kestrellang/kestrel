// CowBox[T] - copy-on-write box built on RcBox

module std.memory

import std.core.(Bool, Cloneable)
import std.memory.(RcBox)

/// Copy-on-write wrapper around `RcBox[T]`.
///
/// Mutable owners use `CowBox`; read-only shared owners (like
/// `StringSlice`) hold the inner `RcBox` directly via `shareBox()`.
/// The mutation protocol is `write()` → modify → `setValue()`.
///
/// # Examples
///
/// ```
/// var box = CowBox(MyStorage());
/// var s = box.write();   // COW barrier — clones if shared
/// s.len = s.len + 1;
/// box.setValue(s);        // write back
/// ```
///
/// # Representation
///
/// A single `RcBox[T]` field.
///
/// # Memory Model
///
/// Same as `RcBox`: non-atomic refcount. Cloning bumps the count;
/// `write` splits off a private copy when shared.
public struct CowBox[T]: Cloneable where T: Cloneable {
    private var inner: RcBox[T]

    /// @name From Value
    /// Allocates fresh storage holding `value` with refcount 1.
    public init(consuming value: T) {
        self.inner = RcBox(value);
    }

    /// @name Inner
    /// Adopts an existing `RcBox` without allocating.
    public init(inner box: RcBox[T]) {
        self.inner = box;
    }

    /// Read access — clones the value so the caller gets an independent
    /// copy. getValue() returns a raw bitwise copy from the heap; cloning
    /// ensures owned resources (byte buffers, etc.) are properly duplicated.
    public func read() -> T {
        self.inner.getValue().clone()
    }

    /// Write access — clones storage if shared, then returns the
    /// (now unique) value. Caller modifies and calls `setValue`.
    public mutating func write() -> T {
        if self.inner.isUnique() == false {
            self.inner = RcBox(self.inner.getValue().clone())
        }
        self.inner.getValue()
    }

    /// Writes `value` into the storage in place. Only valid after
    /// a preceding `write()` call (which ensures uniqueness).
    /// Takes `value` by consuming so the drop pass sees the caller's
    /// local as moved (Dead) — prevents double-free of shared buffers.
    public func setValue(consuming value: T) {
        self.inner.setValue(value)
    }

    /// In-place mutation barrier: ensures unique storage (deep-copying if
    /// shared), then passes the heap value to `body` as a `mutating` argument
    /// to mutate directly — no per-call clone or write-back. This is the O(1)
    /// replacement for the `read()` → modify → `setValue()` dance.
    public mutating func modify[R](body: (mutating T) -> R) -> R {
        if self.inner.isUnique() == false {
            self.inner = RcBox(self.inner.getValue().clone())
        }
        self.inner.modify(body)
    }

    /// Returns a pointer to the wrapped value on the heap, bypassing
    /// the clone that `read()` / `getValue()` would create. Use this
    /// to read individual scalar fields without triggering `T.deinit`.
    public func valuePtr() -> Pointer[T] {
        self.inner.valuePtr()
    }

    /// Returns `true` when no other clone shares this storage.
    public func isUnique() -> Bool {
        self.inner.isUnique()
    }

    /// Returns a shared `RcBox` pointing at the same storage
    /// (refcount bumped). Use this to hand read-only access to
    /// types like `StringSlice`.
    public func shareBox() -> RcBox[T] {
        self.inner.clone()
    }

    /// Shares storage with the returned clone (refcount bump).
    public func clone() -> CowBox[T] {
        CowBox(inner: self.inner.clone())
    }
}
