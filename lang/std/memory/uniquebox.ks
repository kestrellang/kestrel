// UniqueBox[T] - single-owner heap box for one-shot (`consuming`) closures.

module std.memory

import std.core.(Bool, Copyable, fatalError)
import std.numeric.(Int64)
import std.result.(Optional)
import std.memory.(Layout, Pointer, RawPointer, Allocator, SystemAllocator)

// Storage block backing a UniqueBox: a liveness word next to the value, in a
// single allocation. The word is 1 while the payload is present and 0 once
// `takeValue()` has moved it out, so `destroy()` knows whether the slot still
// owns anything. It is an `Int64` (not a `Bool`) for the same reason
// `RcBoxStorage.refCount` is: the payload sits at a fixed `sizeof[Int64]`
// offset, which `valuePtr()` computes without a field-offset intrinsic.
//
// `where T: not Copyable` is a RELAXATION, not a requirement: the payload is
// a consuming closure's environment struct, which owns its captures.
struct UniqueBoxStorage[T]: not Copyable where T: not Copyable {
    var live: Int64
    var value: T
}

/// Heap allocation with exactly one owner, used for the environment of a
/// `consuming` (one-shot) closure.
///
/// Unlike `RcBox` there is no reference count and no sharing: the single
/// owner either *takes* the payload out (`takeValue()`, the one call a
/// `consuming` closure gets) or *destroys* it (`destroy()`, the drop of a
/// closure that was never called). Exactly one of the two runs, and both
/// free the block.
///
/// This is deliberately NOT a `SharedBox` conformer. A shared box releases by
/// dropping its whole payload, which would double-free the capture slots a
/// one-shot body already moved out; the point of the unique box is that the
/// payload leaves the allocation intact and its per-slot teardown then happens
/// in the caller's frame, where the ordinary partial-move machinery applies.
///
/// # Examples
///
/// ```
/// let box = UniqueBox(value: makeResource());
/// let r = box.takeValue();   // moves the payload out and frees the block
/// ```
///
/// # Representation
///
/// One `Pointer[UniqueBoxStorage[T]]`. The pointed-to block holds an `Int64`
/// liveness word followed by the `T` value, allocated via `SystemAllocator`.
///
/// # Memory Model
///
/// Single owner, no counting. `UniqueBox` has no `deinit` on purpose: it is a
/// compiler-internal handle whose lifetime is driven by the closure value that
/// carries it, and dropping the handle value itself must never free the block
/// (the raw pointer word is forgotten and reconstituted across the closure
/// representation). Every allocation is released by exactly one `takeValue()`
/// or `destroy()`.
///
/// # Guarantees
///
/// - `takeValue()` transfers ownership of the payload to the caller and never
///   runs `T.deinit`.
/// - `destroy()` runs `T.deinit` exactly once when the payload is still
///   present, and never when it has already been taken.
@builtin(.UniqueBox)
public struct UniqueBox[T]: not Copyable where T: not Copyable {
    // `fileprivate` mirrors `RcBox.ptr`: same-file extensions may reach it.
    fileprivate var ptr: Pointer[UniqueBoxStorage[T]]

    /// @name From Value
    /// Allocates fresh storage holding `value`. Panics if the underlying
    /// `SystemAllocator` returns `.None`.
    ///
    /// # Errors
    ///
    /// Panics with `"UniqueBox allocation failed"` on allocation failure.
    public init(consuming value: T) {
        let layout: Layout = Layout.of[UniqueBoxStorage[T]]();
        var allocator: SystemAllocator = SystemAllocator();
        let result: RawPointer? = allocator.allocate(layout);
        if let .Some(rawPtr) = result {
            self.ptr = rawPtr.cast[UniqueBoxStorage[T]]();
            self.ptr.write(UniqueBoxStorage(live: 1, value: value));
        } else {
            fatalError("UniqueBox allocation failed")
        }
    }

    /// Pointer to the payload slot inside the block. Mirrors
    /// `RcBox.valuePtr()`: the header is one `Int64`, so the payload starts at
    /// `sizeof[Int64]` bytes.
    fileprivate func valuePtr() -> Pointer[T] {
        let valueOffset = Int64(intLiteral: lang.sizeof[Int64]());
        self.ptr.asRaw().offset(by: valueOffset).cast[T]()
    }

    /// Moves the payload out of the block, leaving the block itself alive and
    /// marked empty. The caller becomes the sole owner of the returned value;
    /// `T.deinit` does not run here.
    ///
    /// The block is NOT freed: a `consuming` closure's call function takes the
    /// environment out this way, and the closure value's release shim
    /// (`destroy()`) reclaims the now-empty block afterwards. That keeps
    /// exactly one free per allocation whether or not the closure was ever
    /// called.
    ///
    /// # Safety
    ///
    /// Must be called at most once per box, and never after `destroy()`.
    public func takeValue() -> T {
        let livePtr = self.ptr.asRaw().cast[Int64]();
        livePtr.write(0);
        self.valuePtr().take()
    }

    /// Drops the payload (if it is still present) and frees the block. This is
    /// the box's single reclamation point — it runs exactly once, whether or
    /// not `takeValue()` emptied the block first.
    ///
    /// # Safety
    ///
    /// Must be called exactly once per box, after any `takeValue()`.
    public func destroy() {
        let livePtr = self.ptr.asRaw().cast[Int64]();
        if livePtr.read() == 1 {
            self.valuePtr().dropInPlace();
        };
        self.free()
    }

    // Releases the block without touching the payload.
    private func free() {
        let layout: Layout = Layout.of[UniqueBoxStorage[T]]();
        var allocator: SystemAllocator = SystemAllocator();
        allocator.deallocate(self.ptr.asRaw(), layout)
    }
}
