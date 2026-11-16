// CowBox[T, A] - copy-on-write box built on RcBox

module std.memory

import std.core.(Bool, Cloneable, Copyable, Indirection, MutableIndirection, Sized, Static)
import std.memory.(Box, RcBox, Allocator, GlobalAllocator)
import std.core.(Defaultable)

/// Copy-on-write wrapper around `RcBox[T, A]`: value semantics on the heap.
///
/// Copies share storage until one of them writes. `value` is the boxed
/// value in place: reading borrows it, and writing forks a shared box first.
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
/// A single `RcBox[T, A]` field.
///
/// # Memory Model
///
/// Same as `RcBox`: non-atomic refcount. Cloning bumps the count; a write
/// through `value` splits off a private copy when shared.
// `T: not Sized`: `any[CowBox] P` holds a raw existential (when `P` refines
// `Cloneable`). Members that move a `T` say `where T: Sized`. `not Static`
// likewise: `CowBox[T, A]` is `Static` exactly when `T` is.
public struct CowBox[T, A = GlobalAllocator]: Cloneable where T: Cloneable, T: not Sized, T: not Static, A: Allocator, A: Copyable {
    // `fileprivate` (not `private`) so the same-file `extend CowBox:
    // MutableIndirection` below can run the COW barrier on `inner`.
    fileprivate var inner: RcBox[T, A]

    /// @name Value and Allocator
    /// Allocates storage from `allocator` holding `value` with refcount 1.
    public init(consuming value: T, allocator allocator: A) where T: Sized {
        self.inner = RcBox(value, allocator: allocator);
    }

    /// @name Inner
    /// Adopts an existing `RcBox` without allocating.
    public init(inner box: RcBox[T, A]) {
        self.inner = box;
    }

    /// COW barrier — makes the storage uniquely owned, cloning it if shared.
    ///
    /// The fork clones the heap value in place (`RcBox.fork`), so a
    /// shared box is deep-copied exactly once and an unsized value works.
    /// Every write (`value`'s `mutating ref`) goes through here.
    ///
    /// Always inlined: every `Array`/`Dictionary` write runs it, and the
    /// unique case is one load and compare. The fork stays out of line
    /// (`forkStorage`) so inlining doesn't pull the deep copy into callers.
    @inline(.Always)
    public mutating func ensureUnique() {
        if not self.inner.isUnique {
            self.forkStorage()
        }
    }

    /// Slow path of `ensureUnique`: replaces the shared box with a private copy.
    @inline(.Never)
    private mutating func forkStorage() {
        self.inner = self.inner.fork()
    }

    /// Pointer to the value on the heap. Internal plumbing for the
    /// accessors below; writing through it skips the fork.
    internal func valuePtr() -> Pointer[T] {
        self.inner.valuePtr()
    }

    /// The boxed value, in place. Reading borrows the shared storage; writing
    /// (`c.value.len = 0`, a `mutating` argument, a mutating method) forks a
    /// shared box first, so other copies never see the change.
    public var value: T {
        ref { unsafe self.inner.valuePtr().value }
        mutating ref {
            self.ensureUnique();
            unsafe self.inner.valuePtr().mutatingValue
        }
    }

    /// `true` when no other copy shares this storage.
    public var isUnique: Bool { self.inner.isUnique }

    /// Shares storage with the returned clone (refcount bump).
    public func clone() -> CowBox[T, A] {
        CowBox(inner: self.inner.clone())
    }
}

// Transparent member access with copy-on-write: a READ peels through
// `value`'s `ref` (no fork), while any WRITE / mutating-method peels through
// its `mutating ref`, which runs the COW barrier first — so `cow.field = v`
// forks a shared box exactly like `cow.value.field = v` does.
//
// The write barrier forks by cloning the value into fresh storage
// (`RcBox.fork`), which works for an unsized value too: an
// existential's table carries its size and clone.
extend CowBox[T, A]: Indirection {
    type Target = T
}

extend CowBox[T, A] where A: Defaultable {
    /// @name From Value
    /// Allocates fresh storage from a default-made `A` holding `value` with
    /// refcount 1.
    public init(consuming value: T) where T: Sized {
        self.inner = RcBox(value);
    }
}

// `any[CowBox] P`: the `init(consuming:)` above is the witness.
extend CowBox[T, A]: Box where A: Defaultable {}

extend CowBox[T, A]: MutableIndirection where T: Cloneable {}
