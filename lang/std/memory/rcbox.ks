// RcBox[T] - reference-counted box for COW (copy-on-write) semantics

module std.memory

import std.core.(Bool, Cloneable, Copyable, fatalError, Indirection, MutableIndirection)
import std.numeric.(Int64)
import std.result.(Optional)
import std.memory.(Layout, Pointer, RawPointer, Allocator, SystemAllocator, SharedBox)

// Storage block backing an RcBox: the refcount lives next to the value,
// in a single allocation, so clones bump a counter rather than copying T.
// `where T: not Copyable` is a RELAXATION, not a requirement: the payload may
// be non-Copyable (an escaping closure's environment struct, a resource
// handle), and Copyable payloads still work. The block itself is never copied.
struct RcBoxStorage[T]: not Copyable where T: not Copyable {
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
/// if b.isUnique() { ... } else { let c = b.deepClone(); /* ... */ }
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
/// - `isUnique()` returning `true` means in-place mutation is safe; this is
///   how COW types decide whether to copy.
/// - The refcount is currently **not** atomic, so `RcBox` is not safe to
///   share across threads.
/// - The payload may be non-`Copyable` (`where T: not Copyable` is a
///   relaxation, the same shape `Pointer[T]` uses): implicit boxing needs it,
///   because a boxed closure environment owns its captures. The two
///   operations that copy the payload back out — `getValue()` and
///   `deepClone()` — are therefore only available when `T: Copyable`.
@builtin(.SharedBox)
public struct RcBox[T]: Cloneable where T: not Copyable {
    // `fileprivate` (not `private`) so the same-file extensions can reach it:
    // the conditional `getValue`/`deepClone` (a conditional member can't sit in
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
    public init(consuming value: T) {
        let layout: Layout = Layout.of[RcBoxStorage[T]]();
        var allocator: SystemAllocator = SystemAllocator();
        let result: RawPointer? = allocator.allocate(layout);
        if let .Some(rawPtr) = result {
            self.ptr = rawPtr.cast[RcBoxStorage[T]]();
            self.ptr.write(RcBoxStorage(refCount: 1, value: value));
        } else {
            fatalError("RcBox allocation failed")
        }
    }

    // Private init used by clone(): adopts an existing storage block
    // (which has already been refcount-bumped) without allocating.
    private init(inner inner: Pointer[RcBoxStorage[T]]) {
        self.ptr = inner;
    }

    /// Mutates the wrapped value in place, passing it to `body` as a
    /// `mutating` argument. No clone or write-back — `body` mutates the
    /// heap value directly. Safe only when this is the unique owner
    /// (`isUnique() == true`); COW types check that first (see `CowBox.modify`).
    public func modify[R](body: (mutating T) -> R) -> R {
        self.valuePtr().withMut(body)
    }

    /// Returns a pointer to the wrapped value on the heap. The pointer
    /// is valid as long as the RcBox (and its storage) is alive. Use
    /// this to read individual fields without creating a full `T` clone
    /// whose deinit would free owned resources prematurely.
    public func valuePtr() -> Pointer[T] {
        let valueOffset = Int64(intLiteral: lang.sizeof[Int64]());
        self.ptr.asRaw().offset(by: valueOffset).cast[T]()
    }

    /// Overwrites the wrapped value in place. Safe only when this is the
    /// unique owner (`isUnique() == true`); otherwise other clones see the
    /// new value, defeating COW. The COW types check `isUnique` before
    /// calling this and `deepClone` otherwise.
    /// Takes `value` by consuming — the caller's copy is dead after this.
    public func setValue(consuming value: T) {
        // Drop the heap occupant before overwriting. Callers always pass a
        // freshly cloned/constructed `value` (read()/getValue() clone; grow()
        // builds a new storage), so the slot's prior value owns a DIFFERENT
        // buffer that `write` (a non-dropping raw store) would otherwise orphan
        // → memory leak on every COW mutation. No caller passes a `value`
        // aliasing the occupant, so dropping first cannot use-after-free.
        self.valuePtr().dropInPlace();
        self.valuePtr().write(value);
    }

    /// Returns `true` when no other clone is sharing storage. The litmus
    /// test for "safe to mutate in place" in COW collections.
    public func isUnique() -> Bool {
        self.ptr.with { (storage) in storage.refCount == 1 }
    }

    /// Current strong reference count. Mostly useful for tests and
    /// diagnostics; production COW logic should branch on `isUnique`.
    public func refCount() -> Int64 {
        self.ptr.with { (storage) in storage.refCount }
    }

    /// Bumps the refcount and returns a second `RcBox` pointing at the
    /// same storage. The receiver and the returned box now both reference
    /// the value; the next mutation should test `isUnique`.
    public func clone() -> RcBox[T] {
        let rcPtr = self.ptr.asRaw().cast[Int64]();
        let count = rcPtr.read();
        rcPtr.write(count + 1);
        RcBox(inner: self.ptr)
    }

    // Drop one reference; deallocate storage when the count hits zero.
    // Called from deinit; not exposed publicly.
    private func release() {
        let rcPtr = self.ptr.asRaw().cast[Int64]();
        let count = rcPtr.read();
        let newCount = count - 1;

        if newCount == 0 {
            // Drop the value field in-place at the heap address, then free the block.
            // RcBoxStorage layout: [refCount: Int64, value: T]
            let valueOffset = Int64(intLiteral: lang.sizeof[Int64]());
            self.ptr.asRaw().offset(by: valueOffset).cast[T]().dropInPlace();
            let layout = Layout.of[RcBoxStorage[T]]();
            var allocator = SystemAllocator();
            allocator.deallocate(self.ptr.asRaw(), layout)
        } else {
            rcPtr.write(newCount)
        }
    }

    /// Decrements the refcount; deallocates storage when it reaches zero.
    deinit {
        self.release()
    }
}

// The two operations that copy the payload OUT of storage. They need
// `T: Copyable` and therefore cannot sit in the struct body, which is relaxed
// to `T: not Copyable`. Every COW client (`CowBox[T] where T: Cloneable`,
// String/Array/Dictionary/Set) satisfies the bound, so this is invisible to
// them; only a box over a non-Copyable payload loses these two methods.
extend RcBox[T] where T: Copyable {
    /// Reads the wrapped value out of storage. Returns a bitwise copy read
    /// straight from the payload slot, so no temporary `RcBoxStorage` is
    /// created or dropped and `T.deinit` does not run.
    public func getValue() -> T {
        self.valuePtr().pointee
    }

    /// Allocates fresh storage with a copy of the value. Used by COW
    /// types when `isUnique()` returns `false` — splits off a private
    /// copy so the caller can mutate without affecting other clones.
    public func deepClone() -> RcBox[T] {
        RcBox(self.valuePtr().pointee)
    }
}

// Transparent member access: `rc.field` reaches the boxed value via the
// heap pointee. `pointeeRef`/`pointeeMutRef` are `&T`/`&mutating T` views into
// the shared storage — NOT through `getValue()` (which would copy out). The
// wrapper still wins name clashes (`rc.clone()` is RcBox.clone, the peel is
// lazy).
extend RcBox[T]: MutableIndirection {
    type Target = T
    public func pointeeRef() -> &T { self.valuePtr().value }
    public mutating func pointeeMutRef() -> &mutating T { self.valuePtr().mutatingValue }
}

// The shared-ownership contract (docs/design/shared-box.md). Most of it is
// already here: `init(consuming value: T)`, `clone()`, `isUnique()` and
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
    public func sharedMutRef() -> &mutating T { self.valuePtr().mutatingValue }

    /// Do the two handles point at the same storage block? Compares raw
    /// storage addresses, the same way `Pointer.isEqual(to:)` does — payload
    /// equality is irrelevant, two independently allocated boxes holding equal
    /// values are never identical.
    public func isIdentical(to other: RcBox[T]) -> Bool {
        self.ptr.address == other.ptr.address
    }
}
