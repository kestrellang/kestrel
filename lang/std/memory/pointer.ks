// Pointer types

module std.memory

import std.ffi.(FFISafe)
import std.core.(Equatable, Bool, Hashable, Hasher, Range, ClosedRange, fatalError, Indirection, MutableIndirection, Sized, ArrayMatchable)
import std.numeric.(Int64, UInt64, UInt8)
import std.memory.(UnsafeSlice)
import std.result.(Optional)
import std.iter.(Iterator, Iterable)
import std.collections.(Sliceable)

/// Untyped pointer to raw memory — `void*` in C terms.
///
/// Used at FFI boundaries and as an intermediate when casting between
/// typed pointers. `RawPointer` deliberately exposes no read/write methods
/// of its own; cast to `Pointer[T]` first via `cast[T]()`. Equality and
/// hashing are address-based.
///
/// # Examples
///
/// ```
/// let p = RawPointer.nullPointer();
/// p.isNull                                // true
/// let typed: Pointer[Int64] = p.cast[Int64]()
/// ```
///
/// # Representation
///
/// One `lang.ptr[lang.i8]`. FFI-safe — passes as a single machine pointer.
public struct RawPointer: Equatable, FFISafe, Hashable {
    /// The wrapped primitive `i8*`.
    public var raw: lang.ptr[lang.i8]

    /// @name From Raw
    /// Wraps an existing primitive pointer.
    public init(raw raw: lang.ptr[lang.i8]) {
        self.raw = raw;
    }

    /// @name From Address
    /// Reconstructs a pointer from a numeric address. Useful for
    /// platform-specific encodings (handles, MMIO addresses); incorrect
    /// addresses produce a pointer that dereferences to undefined memory.
    public init(address address: UInt64) {
        self.raw = lang.ptr_from_address(address.raw)
    }

    /// Returns the canonical null pointer.
    public static func nullPointer() -> RawPointer {
        RawPointer(raw: lang.ptr_null())
    }

    /// Numeric address of the pointee. Round-trips through
    /// `RawPointer(address:)`.
    public var address: UInt64 {
        UInt64(intLiteral: lang.ptr_to_address(self.raw))
    }

    /// Convenience for `address == 0`.
    public var isNull: Bool {
        Bool(boolLiteral: lang.ptr_is_null(self.raw))
    }

    /// Reinterprets the address as a `Pointer[T]`.
    ///
    /// # Safety
    ///
    /// The caller must ensure the address holds a valid `T` (correct size,
    /// alignment, and initialised contents) before reading through the
    /// returned pointer.
    public func cast[T]() -> Pointer[T] where T: not Copyable, T: not Sized, T: not Static {
        Pointer(raw: lang.cast_ptr[_, T](self.raw))
    }

    /// Adds `bytes` to the address (no element-size scaling — this is
    /// raw byte arithmetic). Use `Pointer[T].offset` for element-typed
    /// strides.
    public func offset(by bytes: Int64) -> RawPointer {
        RawPointer(raw: lang.ptr_offset(self.raw, bytes.raw))
    }

    /// Address-based equality. Two `RawPointer`s pointing into different
    /// allocations are equal iff their addresses coincide.
    public func isEqual(to other: RawPointer) -> Bool {
        self.address == other.address
    }

    /// Hashes the underlying address.
    ///
    /// Heap allocations cluster on alignment boundaries, so the raw
    /// address has predictable low bits. We run the address through
    /// Murmur3's `fmix64` finalizer (two rounds of `xor-shift /
    /// multiply`) before hashing so every input bit avalanches across
    /// the 64-bit output. Without this, pointer-keyed maps see
    /// collision clustering driven by the allocator's stride.
    public func hash[H](mutating into hasher: H) where H: Hasher {
        let m1 = 18397679294719823053;  // 0xff51afd7ed558ccd
        let m2 = 14181476777654086739;  // 0xc4ceb9fe1a85ec53
        var x = self.address;
        x = x.bitwiseXor(x.shiftRight(by: 33));
        x = x.multiply(m1);
        x = x.bitwiseXor(x.shiftRight(by: 33));
        x = x.multiply(m2);
        x = x.bitwiseXor(x.shiftRight(by: 33));
        hasher.write(UnsafeSlice(pointer: Pointer(to: x).asRaw().cast[UInt8](), count: 8))
    }
}

/// Typed pointer to a single value of `T`.
///
/// Element-typed counterpart to `RawPointer`: `offset(by:)` strides in
/// units of `sizeof[T]`, and `value` / `mutatingValue` borrow the pointee in place.
/// `Pointer[T]` is FFI-safe when `T` is.
///
/// # Examples
///
/// ```
/// var x = 42;
/// let p = Pointer(to: x);
/// p.read()                       // 42
/// p.write(100)                   // x is now 100
/// p.pointee = 7                  // x is now 7
/// ```
///
/// # Representation
///
/// One `lang.ptr[T]`. The wrapping struct is purely a typing convenience —
/// it lowers to a bare machine pointer.
///
/// # Memory Model
///
/// Non-owning. The pointee's lifetime is the caller's responsibility; the
/// pointer does not increment any refcount, register with any GC, or
/// trigger a deinit.
// `T: not Static` (references 2a): the pointee may be reference-bearing —
// Pointer is the unsafe escape hatch, and Pointer[T] itself stores only a
// raw address (no T), so it stays Static regardless of T.
// `T: not Sized` (existentials): the pointee may be a raw existential, whose
// size is only known at runtime. Members that move a `T` or stride by its
// size say `where T: Sized`.
public struct Pointer[T]: Equatable, Hashable where T: not Copyable, T: not Static, T: not Sized {
    // `fileprivate`, not `private`: the conditional members live in
    // extensions (a conditional member can't sit in the struct body), and an
    // extension can only reach file-scoped members.
    fileprivate var _raw: lang.ptr[T]

    /// The wrapped primitive pointer.
    public var raw: lang.ptr[T] { self._raw }

    /// @name From Raw
    /// Wraps an existing primitive pointer.
    public init(raw raw: lang.ptr[T]) {
        self._raw = raw;
    }

    /// @name To Value
    /// Takes the address of `value`. Equivalent to `&value` in C — the
    /// borrowed place itself is captured; no copy is made.
    ///
    /// # Safety
    ///
    /// The pointer does not keep `value` alive: the caller must ensure the
    /// place outlives every use of the resulting pointer, or any read is
    /// undefined behavior.
    ///
    /// This is the sole capture init and it accepts any place — `var` or
    /// `let` — yielding the same write-capable `Pointer[T]`. Writing
    /// through a pointer captured from an immutable place is the C
    /// const-cast footgun: it compiles, and it is on the caller to know the
    /// storage is actually mutable.
    public init(to value: T) where T: Sized {
        self._raw = lang.ptr_to(value)
    }

    /// Returns a typed null pointer.
    public static func nullPointer() -> Pointer[T] {
        Pointer(raw: lang.ptr_null[T]())
    }

    /// Live view of the value at the address. `get` reads through the
    /// pointer; `set` writes. Both are unchecked — see `# Safety`.
    ///
    /// # Safety
    ///
    /// The pointer must be non-null and the storage must hold a valid
    /// initialised `T`. Reading past the end of an allocation, after
    /// the pointee has been freed, or through a dangling pointer is
    /// undefined behavior.
    /// Numeric address — same value as `asRaw().address`.
    public var address: UInt64 {
        UInt64(intLiteral: lang.ptr_to_address(lang.cast_ptr[_, lang.i8](self._raw)))
    }

    /// Convenience for `address == 0`.
    public var isNull: Bool {
        Bool(boolLiteral: lang.ptr_is_null(lang.cast_ptr[_, lang.i8](self._raw)))
    }

    /// Bit-copies `T` out of the address — `T.deinit` does not run. This is
    /// a copy, so it requires `T: Copyable`; for non-Copyable pointees use
    /// `value` (borrow) or `take` (consuming read-out).
    public func read() -> T where T: Copyable {
        lang.ptr_read(self._raw)
    }

    /// Moves `T` out of the address bitwise — a *consuming* read-out. Unlike
    /// `read()` there is no `Copyable` requirement: ownership of the value
    /// transfers to the caller, and the pointee is left logically
    /// uninitialised. `T.deinit` does not run here.
    ///
    /// This is the dual of `write(consuming:)` and the building block for
    /// relocating non-Copyable values (e.g. `swap`).
    ///
    /// # Safety
    ///
    /// Same validity preconditions as `read()`, and additionally: after the
    /// take the pointee must not be read or dropped until the slot is
    /// re-initialised (e.g. via `write`). Taking the same slot twice without
    /// an intervening write double-owns the value and will double-free.
    public func take() -> T where T: Sized {
        lang.ptr_take(self._raw)
    }

    /// The pointee, in place — no copy, no clone, no `T: Copyable`
    /// requirement. Member access, operators, and borrow-convention calls
    /// go through it in place; binding it (`let x = p.value;`) stores an
    /// owned copy instead (binding decay). Writing through it (`p.value = x`,
    /// a mutating method) needs a `var` pointer; `mutatingValue` writes
    /// through any pointer.
    ///
    /// # Safety
    ///
    /// - The pointer must be non-null and the storage must hold a valid,
    ///   initialized `T` for as long as the reference is used.
    /// - The reference inherits this pointer's contract: the compiler does
    ///   not verify the pointee's lifetime. Using it after the storage is
    ///   freed is undefined behavior — the same trust point as `read()`,
    ///   returning a view instead of a copy.
    public var value: T {
        ref { lang.ptr_ref(self._raw) }
        mutating ref { self.mutatingValue }
    }

    /// Mutable borrowed view of the pointee. Mutating methods and
    /// `mutating`-convention arguments through the result write the storage
    /// in place — no clone, no write-back.
    ///
    /// # Safety
    ///
    /// - Same validity preconditions as `value`.
    /// - This is the const-cast escape hatch: `Pointer(to: x).mutatingValue`
    ///   on a `let`/shared place compiles and writes through it — the
    ///   compiler does not stop it (same class as `write()` after
    ///   `Pointer(to:)` on a `let`).
    /// - Writing through a pointer into storage shared by a COW container
    ///   (`Array`, `String`, `Dictionary`) is visible through every copy,
    ///   breaking value semantics. Make the storage unique first.
    public var mutatingValue: &mutating T {
        lang.ptr_mut_ref(self._raw)
    }

    /// Writes `value` through the pointer, without dropping the old pointee.
    ///
    /// # Safety
    ///
    /// The pointer must be non-null and the storage valid for writes of
    /// `T`. The previous pointee is overwritten without running its
    /// `deinit`. If the pointer was captured with `Pointer(to:)` from a
    /// `let` place, writing is the documented const-cast footgun — the
    /// compiler does not stop it.
    public func write(consuming value: T) where T: Sized {
        lang.ptr_write(self._raw, value)
    }

    /// Replaces an INITIALIZED pointee: runs the old value's destructor in
    /// place, then writes `value`. Use this — not `write` — to overwrite a
    /// live slot; `write` is for uninitialized (or moved-out) storage and
    /// drops nothing, so overwriting a live heap value with it leaks.
    public func assign(consuming value: T) where T: Sized {
        lang.drop_in_place(self._raw);
        lang.ptr_write(self._raw, value)
    }

    /// Runs T's destructor at this address without copying the value to stack.
    /// The pointer remains valid but the pointee is left in a destroyed state.
    public func dropInPlace() {
        lang.drop_in_place(self._raw)
    }

    /// Strides the pointer by `n` *elements* (multiplied by `sizeof[T]`).
    /// Compare with `RawPointer.offset`, which strides by raw bytes.
    public func offset(by n: Int64) -> Pointer[T] where T: Sized {
        let byteOffset = n * Int64(intLiteral: lang.sizeof[T]());
        Pointer[T](raw: lang.ptr_offset[T](self._raw, byteOffset.raw))
    }

    /// Drops the type tag, returning a `RawPointer` to the same address.
    public func asRaw() -> RawPointer {
        RawPointer(raw: lang.cast_ptr[_, lang.i8](self._raw))
    }

    /// Reinterprets the address as a `Pointer[U]`.
    ///
    /// # Safety
    ///
    /// Same caveats as `RawPointer.cast` — the storage must be valid for
    /// `U` (size, alignment, contents) at the moment of the read/write.
    public func cast[U]() -> Pointer[U] where U: not Copyable, U: not Sized, U: not Static {
        Pointer(raw: lang.cast_ptr[_, U](lang.cast_ptr[_, lang.i8](self._raw)))
    }

    /// This address, carrying `other`'s pointee metadata.
    ///
    /// A pointer to an unsized pointee (a raw existential) is two words: the
    /// address and the payload's function table. Address arithmetic through
    /// `asRaw()` keeps only the first; this puts the table back. For a sized
    /// `T` it returns `self` unchanged.
    public func withMetadata[U](of other: Pointer[U]) -> Pointer[T]
        where U: not Copyable, U: not Sized, U: not Static
    {
        Pointer(raw: lang.ptr_with_meta[T, U](self._raw, other._raw))
    }

    /// Address-based equality.
    public func isEqual(to other: Pointer[T]) -> Bool {
        self.address == other.address
    }

    /// Hashes the underlying address.
    ///
    /// Heap allocations cluster on alignment boundaries, so the raw
    /// address has predictable low bits. We run the address through
    /// Murmur3's `fmix64` finalizer (two rounds of `xor-shift /
    /// multiply`) before hashing so every input bit avalanches across
    /// the 64-bit output. Without this, pointer-keyed maps see
    /// collision clustering driven by the allocator's stride.
    public func hash[H](mutating into hasher: H) where H: Hasher {
        let m1 = 18397679294719823053;  // 0xff51afd7ed558ccd
        let m2 = 14181476777654086739;  // 0xc4ceb9fe1a85ec53
        var x = self.address;
        x = x.bitwiseXor(x.shiftRight(by: 33));
        x = x.multiply(m1);
        x = x.bitwiseXor(x.shiftRight(by: 33));
        x = x.multiply(m2);
        x = x.bitwiseXor(x.shiftRight(by: 33));
        hasher.write(UnsafeSlice(pointer: Pointer(to: x).asRaw().cast[UInt8](), count: 8))
    }
}

/// `Pointer[T]` is FFI-safe whenever its element type is.
extend Pointer: FFISafe where T: FFISafe {}

/// Unchecked view over a contiguous run of `T` values: a pointer and a
/// count.
///
/// The slice does **not** track or extend the lifetime of the underlying
/// storage, and the compiler doesn't check it: keeping one past its source,
/// or past the source's next change, is a use-after-free. Use it for FFI,
/// `Buffer`s and other memory that isn't an `Array`, and inside one call.
/// To view part of an `Array`, use `xs(1..<3)`, a `&Slice[T]` the
/// compiler keeps from outliving the array (it is still invalidated when
/// the array reallocates).
///
/// # Examples
///
/// ```
/// let arr = [1, 2, 3, 4];
/// let s = arr.asUnsafeSlice();   // valid until `arr` next changes
/// s(checked: 0)                  // .Some(1)
/// s(checked: 99)                 // .None
/// ```
public struct UnsafeSlice[T] {
    fileprivate var ptr: Pointer[T]
    fileprivate var len: Int64

    /// @name From Pointer
    /// Builds a slice from an existing pointer and element count. The
    /// caller is responsible for ensuring `count` elements live at `pointer`.
    public init(pointer pointer: Pointer[T], count count: Int64) {
        self.ptr = pointer;
        self.len = count;
    }

    /// Element count.
    public var count: Int64 { self.len }

    /// `true` when `count == 0`.
    public var isEmpty: Bool { self.len == 0 }

    /// Pointer to the first element. `pointer.offset(by: i)` reaches
    /// element `i` (0-indexed).
    public var pointer: Pointer[T] { self.ptr }

    // subscript(index:), subscript(checked:), subscript(unchecked:),
    // subscript(clamped:), subscript(wrapped:): provided by extend Sliceable[T]

    /// Forward iterator over the elements.
    public func iter() -> UnsafeSliceIterator[T] {
        UnsafeSliceIterator(ptr: self.ptr, remaining: self.len)
    }

    /// First element, or `.None` for an empty slice.
    public func first() -> Optional[T] {
        if self.len > 0 {
            .Some(self.ptr.read())
        } else {
            .None
        }
    }

    /// Last element, or `.None` for an empty slice.
    public func last() -> Optional[T] {
        if self.len > 0 {
            .Some(self.ptr.offset(by: self.len - 1).read())
        } else {
            .None
        }
    }

}

/// `Sliceable[T]` conformance — `UnsafeSlice` is the kernel type, so
/// `asUnsafeSlice()` returns `self`. Also declares the `Iterable` associated
/// types so slices can be used in `for`-`in` loops and generic
/// `I: Iterable` contexts.
extend UnsafeSlice[T]: Sliceable[T], Iterable {
    type Item = T
    type TargetIterator = UnsafeSliceIterator[T]

    /// Returns `self` — `UnsafeSlice` is already the borrowed view.
    public func asUnsafeSlice() -> UnsafeSlice[T] { self }

}

/// Element-wise equality when the element type is `Equatable`. `isEqual`
/// is supplied by `extend Sliceable[T] where T: Equatable` in
/// `std.collections.slice`.
///
/// # Examples
///
/// ```
/// let a = [1, 2, 3].asUnsafeSlice();
/// let b = [1, 2, 3].asUnsafeSlice();
/// a == b;  // true
///
/// let c = [4, 5, 6].asUnsafeSlice();
/// a == c;  // false (same length, different elements)
/// ```
extend UnsafeSlice[T]: Equatable where T: Equatable { }

/// `ArrayMatchable` conformance: array patterns over raw memory. `..rest`
/// binds a narrower `UnsafeSlice` (no lifetime tracking, like the slice).
extend UnsafeSlice[T]: ArrayMatchable {
    type Element = T
    type Rest = UnsafeSlice[T]

    public func matchLength() -> Int64 { self.len }

    public func matchGet(index: Int64) -> T { self.ptr.offset(by: index).read() }

    public func matchSlice(from: Int64, to: Int64) -> UnsafeSlice[T] {
        UnsafeSlice(pointer: self.ptr.offset(by: from), count: to - from)
    }
}

/// Forward iterator over an `UnsafeSlice[T]`. Holds a moving pointer and a
/// remaining count; advancing reads through the pointer.
///
/// # Representation
///
/// A `Pointer[T]` cursor and an `Int64` countdown.
public struct UnsafeSliceIterator[T]: Iterator {
    type Item = T

    private var ptr: Pointer[T]
    private var remaining: Int64

    /// @name From Storage
    /// Builds an iterator from a starting pointer and remaining count.
    public init(ptr ptr: Pointer[T], remaining remaining: Int64) {
        self.ptr = ptr;
        self.remaining = remaining;
    }

    /// Yields the next element, or `.None` when the count reaches zero.
    public mutating func next() -> Optional[T] {
        if self.remaining > 0 {
            let value = self.ptr.read();
            self.ptr = self.ptr.offset(by: 1);
            self.remaining = self.remaining - 1;
            .Some(value)
        } else {
            .None
        }
    }
}

/// Forward iterator yielding SHARED REFERENCES (`&T`) to contiguous
/// elements in place — no copies, no clones. The by-reference sibling of
/// `UnsafeSliceIterator`; surfaced as `Array.refs()`.
///
/// # Invalidation
///
/// Holds a raw pointer into the underlying buffer. Mutating the source
/// collection's STRUCTURE while iterating (append/realloc, removal)
/// invalidates the cursor and any yielded reference — the same contract
/// as every pointer-backed iterator, met through references here.
///
/// # Representation
///
/// A `Pointer[T]` cursor and an `Int64` countdown.
public struct RefSliceIterator[T]: Iterator {
    type Item = &T

    private var ptr: Pointer[T]
    private var remaining: Int64

    /// @name From Storage
    /// Builds an iterator from a starting pointer and remaining count.
    public init(ptr ptr: Pointer[T], remaining remaining: Int64) {
        self.ptr = ptr;
        self.remaining = remaining;
    }

    /// Yields a reference to the next element in place, or `.None` when
    /// the count reaches zero.
    public mutating func next() -> Optional[&T] {
        if self.remaining > 0 {
            let r = &self.ptr.value;
            self.ptr = self.ptr.offset(by: 1);
            self.remaining = self.remaining - 1;
            let o: Optional[&T] = .Some(r);
            o
        } else {
            .None
        }
    }
}

/// Forward iterator yielding MUTABLE REFERENCES (`&mutating T`) to
/// contiguous elements — in-place mutation without writeback
/// (`for x in arr.mutatingRefs() { x += 1 }`). Surfaced as
/// `Array.mutatingRefs()`, which runs the COW barrier before handing out
/// the buffer.
///
/// # Invalidation
///
/// Same contract as `RefSliceIterator`: mutating the source collection's
/// STRUCTURE while iterating invalidates the cursor and any yielded
/// reference. Element writes through the yielded references are the
/// intended use.
///
/// # Representation
///
/// A `Pointer[T]` cursor and an `Int64` countdown.
public struct MutRefSliceIterator[T]: Iterator {
    type Item = &mutating T

    private var ptr: Pointer[T]
    private var remaining: Int64

    /// @name From Storage
    /// Builds an iterator from a starting pointer and remaining count.
    public init(ptr ptr: Pointer[T], remaining remaining: Int64) {
        self.ptr = ptr;
        self.remaining = remaining;
    }

    /// Yields a mutable reference to the next element in place, or
    /// `.None` when the count reaches zero.
    public mutating func next() -> Optional[&mutating T] {
        if self.remaining > 0 {
            let r = &mutating self.ptr.mutatingValue;
            self.ptr = self.ptr.offset(by: 1);
            self.remaining = self.remaining - 1;
            let o: Optional[&mutating T] = .Some(r);
            o
        } else {
            .None
        }
    }
}

// A raw `Pointer[T]` is a smart pointer: `ptr.field` reaches the pointee
// through `value`. Works for any `T` (Copyable or not).
extend Pointer[T]: MutableIndirection {
    type Target = T
}



/// Exchanges the contents of two mutable locations without cloning or
/// dropping either — three bitwise moves, like Rust's `mem::swap`. For a
/// COW value (`Array`, `String`, `Dictionary`) this swaps the *handles*, so
/// it never touches the heap buffers: an `O(1)` pointer exchange regardless
/// of contents.
///
/// This is the idiomatic way to rotate double buffers (`swap(a, b)` instead
/// of the `let tmp = a; a = b; b = tmp` dance, which clones for non-`Copyable`
/// types).
///
/// # Examples
///
/// ```
/// var a = [1, 2, 3];
/// var b = [9, 8];
/// swap(a, b);          // a == [9, 8], b == [1, 2, 3] — no element copies
/// ```
public func swap[T](mutating a: T, mutating b: T) where T: not Copyable, T: not Static {
    let pa = Pointer(to: a);
    let pb = Pointer(to: b);
    // Self-swap would take the same slot twice (double-owning the value and
    // leaking the second copy), so short-circuit aliasing locations.
    if pa.address == pb.address {
        return
    }
    let tmp = pa.take();
    pa.write(pb.take());
    pb.write(tmp)
}
