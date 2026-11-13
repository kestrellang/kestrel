// OwnedBox[T, A] - single-owner heap box that frees its value on drop.

module std.memory

import std.core.(Defaultable, Bool, Cloneable, ConsumingIndirection, Copyable, fatalError, Indirection, MutableIndirection, Sized, Static)
import std.numeric.(Int64)
import std.result.(Optional)
import std.memory.(Box, Layout, Pointer, RawPointer, Allocator, GlobalAllocator)

/// Heap allocation with exactly one owner.
///
/// `OwnedBox` is the unique counterpart of `RcBox`: moving the box moves
/// ownership, dropping it drops the boxed value and frees the block, and there
/// is no sharing. Because no other handle can see the boxed value, the owner may
/// also take it back out (`consumeValue`). That makes it the box to use
/// for an existential whose value must be consumed: `any[OwnedBox] P`.
///
/// # Examples
///
/// ```
/// let box = OwnedBox(File(path: "log.txt"));
/// box.write("hi");       // member access reaches the boxed value
/// // `box` drops here: `File.deinit` runs, then the block is freed
/// ```
///
/// # Representation
///
/// A `Pointer[T]` to a heap block holding just the boxed value, plus the
/// allocator `A` that frees it: one word for a sized value and a
/// stateless allocator. The pointer is null once `consumeValue` has given the
/// value up. The block is measured from the boxed value itself (`Layout.ofValue`),
/// so an unsized value works.
///
/// # Memory Model
///
/// Single owner, no counting. `OwnedBox` is never bit-copied; it is
/// `Cloneable` when its value can be copied at all (bitwise or by its own
/// `clone()`), and a clone allocates a new block holding that copy.
///
/// # Guarantees
///
/// - The value's deinit runs exactly once: when the box drops, unless
///   `consumeValue` took the boxed value out first.
/// - The block is freed exactly once.
//
// `T: not Copyable, not Sized, not Static`: the boxed value may be move-only, a
// raw existential, or reference-bearing. Members that move a `T` say
// `where T: Sized`.
public struct OwnedBox[T, A = GlobalAllocator]: not Copyable where T: not Copyable, T: not Sized, T: not Static, A: Allocator, A: Copyable {
    // Null once `consumeValue` has handed the boxed value out and freed the
    // block, so `deinit` has nothing left to do (Kestrel has no `forget`).
    // No separate liveness flag: indirect enum cases store this box, so
    // every word here is a word in each recursive node.
    fileprivate var ptr: Pointer[T]
    // Frees the block. A stateless allocator adds no bytes, so an
    // indirect enum node still pays one word per box.
    fileprivate var allocator: A

    /// @name Value and Allocator
    /// Moves `value` into a block allocated from `allocator`.
    ///
    /// # Errors
    ///
    /// Panics with `"OwnedBox allocation failed"` on allocation failure.
    public init(consuming value: T, allocator allocator: A) where T: Sized {
        let layout: Layout = Layout.of[T]();
        var alloc = allocator;
        let result: RawPointer? = alloc.allocate(layout);
        self.allocator = alloc;
        if let .Some(rawPtr) = result {
            self.ptr = rawPtr.cast[T]();
            unsafe self.ptr.write(value);
        } else {
            fatalError("OwnedBox allocation failed")
        }
    }

    /// Pointer to the boxed value. Valid while the box is alive.
    internal func valuePtr() -> Pointer[T] {
        self.ptr
    }

    /// The value, in place: `&b.value` borrows it, `b.value.x = 1` writes
    /// it. The box is its only owner, so a write needs a mutable box.
    public var value: T {
        ref { unsafe self.ptr.value }
        mutating ref { unsafe self.ptr.mutatingValue }
    }

    // Frees the block without touching the boxed value. Measured from the
    // value so an unsized one works too.
    fileprivate func free() {
        let layout = unsafe Layout.ofValue(at: self.ptr);
        var allocator = self.allocator;
        unsafe allocator.deallocate(self.ptr.asRaw(), layout)
    }

    /// Drops the boxed value and frees the block, unless `consumeValue`
    /// already took the boxed value out.
    deinit {
        if self.ptr.isNull { return; }
        unsafe self.ptr.dropInPlace();
        self.free()
    }
}

extend OwnedBox[T, A]: MutableIndirection {
    type Target = T
}

// The box owns its value alone, so it can give it up.
extend OwnedBox[T, A]: ConsumingIndirection {
    public consuming func consumeValue[R](body: (Pointer[T]) -> R) -> R where R: not Copyable, R: not Static {
        // Move `self` into a `var` before nulling `ptr`: assigning a field
        // of a `consuming self` directly writes to a copy today.
        var me = self;
        let result = body(me.ptr);
        me.free();
        // Null address, same metadata: `deinit` then skips the boxed value.
        me.ptr = RawPointer.nullPointer().cast[T]().withMetadata(of: me.ptr);
        result
    }
}

extend OwnedBox[T, A] where A: Defaultable {
    /// @name From Value
    /// Allocates a block holding `value` from a default-made `A`.
    ///
    /// # Errors
    ///
    /// Panics with `"OwnedBox allocation failed"` on allocation failure.
    public init(consuming value: T) where T: Sized {
        self.init(value, allocator: A())
    }
}

// `any[OwnedBox] P`: the `init(consuming:)` above is the witness. The box
// is its value's only owner, so it is always unique.
extend OwnedBox[T, A]: Box where A: Defaultable {
    public var isUnique: Bool { true }
}

// A clone allocates a fresh block holding a copy of the boxed value — a bit-copy
// or the boxed value's own `clone()`, whichever copying `T` means.
extend OwnedBox[T, A]: Cloneable where T: Copyable, T: Sized {
    public func clone() -> OwnedBox[T, A] {
        OwnedBox(unsafe self.ptr.value, allocator: self.allocator)
    }
}
