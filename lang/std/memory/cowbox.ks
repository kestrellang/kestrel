// CowBox[T] - copy-on-write box built on RcBox

module std.memory

import std.core.(Bool, Cloneable, Indirection, MutableIndirection, Sized, Static)
import std.memory.(ExistentialBox, RcBox)

/// Copy-on-write wrapper around `RcBox[T]`: value semantics on the heap.
///
/// Copies share storage until one of them writes. `value` is the payload
/// in place: reading borrows it, and writing forks a shared box first.
///
/// # Examples
///
/// ```
/// var box = CowBox(MyStorage());
/// let other = box;          // shares storage
/// box.value.len += 1;       // forks first; `other` is unchanged
/// let r = &box.value;       // borrows, no copy
/// ```
///
/// # Representation
///
/// A single `RcBox[T]` field.
///
/// # Memory Model
///
/// Same as `RcBox`: non-atomic refcount. Cloning bumps the count; a write
/// through `value` splits off a private copy when shared.
// `T: not Sized`: `any[CowBox] P` holds a raw existential (when `P` refines
// `Cloneable`). Members that move a `T` say `where T: Sized`. `not Static`
// likewise: `CowBox[T]` is `Static` exactly when `T` is.
public struct CowBox[T]: Cloneable where T: Cloneable, T: not Sized, T: not Static {
    // `fileprivate` (not `private`) so the same-file `extend CowBox:
    // MutableIndirection` below can run the COW barrier on `inner`.
    fileprivate var inner: RcBox[T]

    /// @name From Value
    /// Allocates fresh storage holding `value` with refcount 1.
    public init(consuming value: T) where T: Sized {
        self.inner = RcBox(value);
    }

    /// @name Inner
    /// Adopts an existing `RcBox` without allocating.
    public init(inner box: RcBox[T]) {
        self.inner = box;
    }

    /// COW barrier — makes the storage uniquely owned, cloning it if shared.
    ///
    /// The fork clones the heap value in place (`RcBox.fork`), so a
    /// shared box is deep-copied exactly once and an unsized payload works.
    /// Every write (`value`'s `mutating ref`, `pointeeMutRef`) goes through
    /// here.
    public mutating func ensureUnique() {
        if not self.inner.isUnique {
            self.inner = self.inner.fork()
        }
    }

    /// Pointer to the payload on the heap. Internal plumbing for the
    /// accessors below; writing through it skips the fork.
    internal func valuePtr() -> Pointer[T] {
        self.inner.valuePtr()
    }

    /// The payload, in place. Reading borrows the shared storage; writing
    /// (`c.value.len = 0`, a `mutating` argument, a mutating method) forks a
    /// shared box first, so other copies never see the change.
    public var value: T {
        ref { self.inner.valuePtr().value }
        mutating ref {
            self.ensureUnique();
            self.inner.valuePtr().mutatingValue
        }
    }

    /// `true` when no other copy shares this storage.
    public var isUnique: Bool { self.inner.isUnique }

    /// Shares storage with the returned clone (refcount bump).
    public func clone() -> CowBox[T] {
        CowBox(inner: self.inner.clone())
    }
}

// Transparent member access with copy-on-write: a READ peels through
// `pointeeRef()` (no fork), while any WRITE / mutating-method peels through
// `pointeeMutRef()`, which runs the COW barrier first — so `cow.field = v`
// forks a shared box exactly like `cow.value.field = v` does.
//
// The write barrier forks by cloning the payload into fresh storage
// (`RcBox.fork`), which works for an unsized payload too: an
// existential's table carries its size and clone.
extend CowBox[T]: Indirection {
    type Target = T
    public func pointeeRef() -> &T { self.valuePtr().value }
}

// `any[CowBox] P`: the struct's `init(consuming:)` is the witness.
extend CowBox[T]: ExistentialBox {}

extend CowBox[T]: MutableIndirection where T: Cloneable {
    public mutating func pointeeMutRef() -> &mutating T {
        self.ensureUnique();
        self.valuePtr().mutatingValue
    }
}
