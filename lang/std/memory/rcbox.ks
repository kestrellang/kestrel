// RcBox[T] - reference-counted box for COW (copy-on-write) semantics

module std.memory

import std.core.(Bool, Cloneable, Copyable, fatalError, Indirection, MutableIndirection, Sized, Static)
import std.numeric.(Int64)
import std.result.(Optional)
import std.memory.(Layout, Pointer, RawPointer, Allocator, SystemAllocator, SharedBox)

// Storage block backing an RcBox: the refcount lives next to the value,
// in a single allocation, so clones bump a counter rather than copying T.
// `where T: not Copyable` is a RELAXATION, not a requirement: the payload may
// be non-Copyable (an escaping closure's environment struct, a resource
// handle), and Copyable payloads still work. The block itself is never copied.
// `T: not Sized`: the payload may be a raw existential — it is the block's
// last field, an unsized tail.
struct RcBoxStorage[T]: not Copyable where T: not Copyable, T: not Sized, T: not Static {
    var refCount: Int64  // TODO: Should be atomic
    var value: T
}

/// Heap allocation with a strong-reference count, used as the underlying
/// storage for the stdlib's copy-on-write types.
///
/// `String`, `Array`, and `Dictionary` all wrap an `RcBox` so that a
/// plain assignment shares storage and only the first mutating call pays
/// for a deep copy. Reach for `RcBox` directly when building a similar
/// COW type; for plain shared ownership without mutation prefer a more
/// purpose-built container.
///
/// `RcBox` is also the `@builtin(.SharedBox)` binding — the box the compiler
/// instantiates for implicit boxing (escaping closure environments today).
/// That binding is what promises deterministic last-release cleanup and no
/// strong-cycle collection; the `SharedBox` protocol itself promises neither.
///
/// # Examples
///
/// ```
/// let a = RcBox(value: [1, 2, 3]);
/// let b = a.clone();          // shares storage; refCount == 2
/// b.mutatingValue.append(4);  // every handle sees it
/// &a.value;                   // read-only borrow: [1, 2, 3, 4]
/// ```
///
/// # Representation
///
/// One `Pointer[RcBoxStorage[T]]`. The pointed-to block holds an `Int64`
/// refcount followed by the `T` value, allocated via `SystemAllocator`.
///
/// # Memory Model
///
/// Reference-counted, non-atomic (today — see TODOs). `clone()` increments
/// the count and shares storage; `deinit` decrements and frees on zero.
/// `deepClone()` allocates a fresh `RcBox` carrying a copied value.
///
/// # Guarantees
///
/// - `isUnique` being `true` means in-place mutation is safe; this is
///   how COW types decide whether to copy.
/// - The refcount is currently **not** atomic, so `RcBox` is not safe to
///   share across threads.
/// - The payload may be non-`Copyable` (`where T: not Copyable` is a
///   relaxation, the same shape `Pointer[T]` uses): implicit boxing needs it,
///   because a boxed closure environment owns its captures. The two
///   `deepClone()`, which copies the payload out, is therefore only
///   available when `T: Copyable`.
///
/// `T: not Sized`: an `RcBox` can hold a raw existential — `any P` is
/// `RcBox[<raw any P>]` by default. Members that move a `T` say
/// `where T: Sized`; everything else works on any payload. `T: not Static`
/// for the same reason: a raw existential is only `Static` when its
/// protocol is, so `RcBox[T]` is `Static` exactly when `T` is.
@builtin(.SharedBox)
public struct RcBox[T]: Cloneable where T: not Copyable, T: not Sized, T: not Static {
    // `fileprivate` (not `private`) so the same-file extensions can reach it:
    // the conditional `deepClone` (a conditional member can't sit in
    // the struct body) and `extend RcBox: SharedBox`, which compares storage
    // addresses for `isIdentical(to:)`.
    fileprivate var ptr: Pointer[RcBoxStorage[T]]

    /// @name From Value
    /// Allocates fresh storage holding `value` with refcount 1. Panics if
    /// the underlying `SystemAllocator` returns `.None`.
    ///
    /// # Errors
    ///
    /// Panics with `"RcBox allocation failed"` on allocation failure.
    public init(consuming value: T) where T: Sized {
        let layout: Layout = Layout.of[RcBoxStorage[T]]();
        var allocator: SystemAllocator = SystemAllocator();
        let result: RawPointer? = allocator.allocate(layout);
        if let .Some(rawPtr) = result {
            self.ptr = rawPtr.cast[RcBoxStorage[T]]();
            unsafe self.ptr.write(RcBoxStorage(refCount: 1, value: value));
        } else {
            fatalError("RcBox allocation failed")
        }
    }

    // Used by clone() and fork(): adopts an existing storage block
    // (already refcount-bumped, or fresh with count 1) without allocating.
    fileprivate init(inner inner: Pointer[RcBoxStorage[T]]) {
        self.ptr = inner;
    }

    /// Returns a pointer to the wrapped value on the heap. The pointer
    /// is valid as long as the RcBox (and its storage) is alive. Use
    /// this to read individual fields without creating a full `T` clone
    /// whose deinit would free owned resources prematurely.
    internal func valuePtr() -> Pointer[T] {
        // The payload's table rides on `self.ptr` (existentials): an
        // unsized `T` is reached by (address, table).
        self.ptr.asRaw().offset(by: self.payloadOffset()).cast[T]().withMetadata(of: self.ptr)
    }

    /// The payload, read-only. Writes go through `mutatingValue`, because
    /// every handle onto this storage sees them.
    public var value: &T {
        unsafe self.valuePtr().value
    }

    /// The payload for writing, through a shared handle: no fork, and every
    /// clone observes the write. Works on a `let` handle.
    ///
    /// # Safety
    ///
    /// Sound only while Kestrel is single-threaded.
    //
    // Must stay a direct pointer-peel chain: see `sharedMutRef`.
    public var mutatingValue: &mutating T {
        unsafe self.valuePtr().mutatingValue
    }

    // Byte offset of `value` inside the storage block `[refCount: Int64,
    // value: T]`: the refcount rounded up to the payload's alignment. That is
    // exactly the block's own alignment (max(8, align T)), which
    // `alignof_val` reads at runtime, so it is right for an over-aligned or
    // unsized payload too.
    fileprivate func payloadOffset() -> Int64 {
        Int64(intLiteral: unsafe lang.alignof_val(self.ptr.raw))
    }

    /// `true` when no other clone is sharing storage. The litmus test for
    /// "safe to mutate in place" in COW collections.
    public var isUnique: Bool { self.refCount == 1 }

    /// Current strong reference count. Mostly useful for tests and
    /// diagnostics; production COW logic should branch on `isUnique`.
    public var refCount: Int64 {
        // The refcount is the block's first field; read it directly, so
        // this works whatever (and however sized) the payload is.
        unsafe self.ptr.asRaw().cast[Int64]().read()
    }

    /// Bumps the refcount and returns a second `RcBox` pointing at the
    /// same storage. The receiver and the returned box now both reference
    /// the value; the next mutation should test `isUnique`.
    public func clone() -> RcBox[T] {
        let rcPtr = self.ptr.asRaw().cast[Int64]();
        let count = unsafe rcPtr.read();
        unsafe rcPtr.write(count + 1);
        RcBox(inner: self.ptr)
    }

    // Drop one reference; deallocate storage when the count hits zero.
    // Called from deinit; not exposed publicly.
    private func release() {
        let rcPtr = self.ptr.asRaw().cast[Int64]();
        let count = unsafe rcPtr.read();
        let newCount = count - 1;

        if newCount == 0 {
            // Drop the value field in-place at the heap address, then free the block.
            // Measured from the block itself so an unsized payload works too.
            let layout = unsafe Layout.ofValue(at: self.ptr);
            unsafe self.valuePtr().dropInPlace();
            var allocator = SystemAllocator();
            unsafe allocator.deallocate(self.ptr.asRaw(), layout)
        } else {
            unsafe rcPtr.write(newCount)
        }
    }

    /// Decrements the refcount; deallocates storage when it reaches zero.
    deinit {
        self.release()
    }
}

// Copies the payload out of storage, so it needs `T: Copyable` and cannot
// sit in the struct body, which is relaxed to `T: not Copyable`.
extend RcBox[T] where T: Copyable {
    /// Allocates fresh storage with a copy of the value. Used by COW
    /// types when `isUnique` is `false` — splits off a private
    /// copy so the caller can mutate without affecting other clones.
    public func deepClone() -> RcBox[T] {
        let copy: T = unsafe self.valuePtr().value;
        RcBox(copy)
    }
}

// Copy-on-write's fork: fresh storage holding a clone of the payload, with
// refcount 1. Works for an unsized payload (`any P` with `P: Cloneable`): the
// block is measured from this one and the payload clones through its
// function table (`lang.clone_val_into`).
extend RcBox[T] where T: Cloneable {
    public func fork() -> RcBox[T] {
        let layout = unsafe Layout.ofValue(at: self.ptr);
        var allocator = SystemAllocator();
        let result: RawPointer? = allocator.allocate(layout);
        if let .Some(raw) = result {
            unsafe raw.cast[Int64]().write(1);
            let fresh: RcBox[T] = RcBox(inner: raw.cast[RcBoxStorage[T]]().withMetadata(of: self.ptr));
            unsafe lang.clone_val_into(self.valuePtr().raw, fresh.valuePtr().raw);
            return fresh
        }
        fatalError("RcBox allocation failed")
    }
}

// Transparent member access: `rc.field` reaches the boxed value via the
// heap pointee. `pointeeRef`/`pointeeMutRef` are `&T`/`&mutating T` views into
// the shared storage, never a copy. The
// wrapper still wins name clashes (`rc.clone()` is RcBox.clone, the peel is
// lazy).
extend RcBox[T]: MutableIndirection {
    type Target = T
    public func pointeeRef() -> &T { unsafe self.valuePtr().value }
    public mutating func pointeeMutRef() -> &mutating T { unsafe self.valuePtr().mutatingValue }
}

// The shared-ownership contract (docs/design/shared-box.md). Most of it is
// already here: `init(consuming value: T)`, `clone()`, `isUnique` and
// `deinit` on the struct, `Target`/`pointeeRef()` on the `MutableIndirection`
// extension above — `Target` is inherited through that conformance and must
// NOT be redeclared here. Only the two genuinely new operations live below.
extend RcBox[T]: SharedBox {
    /// Mutable access to the value through a *shared* handle: non-`mutating`
    /// receiver, no copy-on-write barrier, every clone observes the write.
    /// This is `pointeeMutRef` without the fork — the interior-mutability
    /// primitive escaping closures and (later) classes are built on. COW
    /// clients must keep using `CowBox`, which forks first.
    ///
    /// # Safety
    ///
    /// Writing here is visible through every handle onto this storage. Sound
    /// only while Kestrel is single-threaded.
    //
    // The body must stay LITERALLY this direct pointer-peel chain. A
    // non-`mutating` method may return `&mutating` only because the reference's
    // provenance is `PointerDerived{mutable: true}`, and that root is stamped
    // only when EVERY return-position expression is a direct pointer-intrinsic
    // call (`RetRefPointerDerived` is Callee::Direct-only and
    // all-returns-must-qualify). Hoisting the pointer into a `let`, wrapping it
    // in a helper, or adding a second return path loses the provenance and the
    // method starts failing E495. Do not refactor.
    public func sharedMutRef() -> &mutating T { unsafe self.valuePtr().mutatingValue }

    /// Do the two handles point at the same storage block? Compares raw
    /// storage addresses, the same way `Pointer.isEqual(to:)` does — payload
    /// equality is irrelevant, two independently allocated boxes holding equal
    /// values are never identical.
    public func isIdentical(to other: RcBox[T]) -> Bool {
        self.ptr.address == other.ptr.address
    }
}

/// The box `any P` is written in: `any P` is `DefaultBox[<raw existential>]`,
/// so copies of an `any P` share one payload (reference semantics). Rebinding
/// this alias changes every default existential at once; write `any[B] P` to
/// pick a box at one site (docs/plans/existentials/existentials-design.md).
@builtin(.DefaultBoxType)
public type DefaultBox[T] = RcBox[T]
