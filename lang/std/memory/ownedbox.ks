// OwnedBox[T] - single-owner heap box that frees its payload on drop.

module std.memory

import std.core.(Bool, Cloneable, ConsumingIndirection, fatalError, Indirection, MutableIndirection, Sized, Static)
import std.numeric.(Int64)
import std.result.(Optional)
import std.memory.(ExistentialBox, Layout, Pointer, RawPointer, Allocator, SystemAllocator)

/// Heap allocation with exactly one owner.
///
/// `OwnedBox` is the unique counterpart of `RcBox`: moving the box moves
/// ownership, dropping it drops the payload and frees the block, and there
/// is no sharing. Because no other handle can see the payload, the owner may
/// also take it back out (`consumePointee`). That makes it the box to use
/// for an existential whose payload must be consumed: `any[OwnedBox] P`.
///
/// # Examples
///
/// ```
/// let box = OwnedBox(File(path: "log.txt"));
/// box.write("hi");       // member access reaches the payload
/// // `box` drops here: `File.deinit` runs, then the block is freed
/// ```
///
/// # Representation
///
/// A `Pointer[T]` to a heap block holding just the payload, plus a liveness
/// flag. The block is measured from the payload itself (`Layout.ofValue`),
/// so an unsized payload works.
///
/// # Memory Model
///
/// Single owner, no counting. `OwnedBox` is not `Copyable`; it is
/// `Cloneable` when its payload is, and a clone allocates a new block.
///
/// # Guarantees
///
/// - The payload's deinit runs exactly once: when the box drops, unless
///   `consumePointee` took the payload out first.
/// - The block is freed exactly once.
//
// `T: not Copyable, not Sized, not Static`: the payload may be move-only, a
// raw existential, or reference-bearing. Members that move a `T` say
// `where T: Sized`.
public struct OwnedBox[T]: not Copyable where T: not Copyable, T: not Sized, T: not Static {
    fileprivate var ptr: Pointer[T]
    // False once `consumePointee` has handed the payload out and freed the
    // block, so `deinit` has nothing left to do. Kestrel has no `forget`.
    fileprivate var live: Bool

    /// @name From Value
    /// Allocates a block holding `value`. Panics if the underlying
    /// `SystemAllocator` returns `.None`.
    ///
    /// # Errors
    ///
    /// Panics with `"OwnedBox allocation failed"` on allocation failure.
    public init(consuming value: T) where T: Sized {
        let layout: Layout = Layout.of[T]();
        var allocator: SystemAllocator = SystemAllocator();
        let result: RawPointer? = allocator.allocate(layout);
        if let .Some(rawPtr) = result {
            self.ptr = rawPtr.cast[T]();
            self.ptr.write(value);
            self.live = true;
        } else {
            fatalError("OwnedBox allocation failed")
        }
    }

    /// Pointer to the payload. Valid while the box is alive.
    public func valuePtr() -> Pointer[T] {
        self.ptr
    }

    // Frees the block without touching the payload. Measured from the
    // payload so an unsized one works too.
    fileprivate func free() {
        let layout = Layout.ofValue(at: self.ptr);
        var allocator = SystemAllocator();
        allocator.deallocate(self.ptr.asRaw(), layout)
    }

    /// Drops the payload and frees the block, unless `consumePointee`
    /// already took the payload out.
    deinit {
        if self.live {
            self.ptr.dropInPlace();
            self.free()
        }
    }
}

extend OwnedBox[T]: MutableIndirection {
    type Target = T
    public func pointeeRef() -> &T { self.ptr.value }
    public mutating func pointeeMutRef() -> &mutating T { self.ptr.mutatingValue }
}

// The box owns its payload alone, so it can give it up.
extend OwnedBox[T]: ConsumingIndirection {
    public consuming func consumePointee[R](body: (Pointer[T]) -> R) -> R where R: not Copyable, R: not Static {
        // Move `self` into a `var` before clearing `live`: assigning a field
        // of a `consuming self` directly writes to a copy today.
        var me = self;
        let result = body(me.ptr);
        me.free();
        me.live = false;
        result
    }
}

// `any[OwnedBox] P`: the struct's `init(consuming:)` is the witness.
extend OwnedBox[T]: ExistentialBox {}

// Moves the payload out and frees the block.
extend OwnedBox[T] where T: Sized {
    public consuming func takeValue() -> T {
        self.consumePointee { (p) in p.take() }
    }
}

// A clone allocates a fresh block holding a clone of the payload. An unsized
// payload clones through its function table (existentials M4).
extend OwnedBox[T]: Cloneable where T: Cloneable, T: Sized {
    public func clone() -> OwnedBox[T] {
        OwnedBox(self.ptr.value.clone())
    }
}
