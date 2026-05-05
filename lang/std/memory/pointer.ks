// Pointer types

module std.memory

import std.ffi.(FFISafe)
import std.core.(Equatable, Bool, Hashable, Hasher, ArrayMatchable, Range, ClosedRange, fatalError)
import std.numeric.(Int64, UInt64, UInt8)
import std.memory.(ArraySlice)
import std.result.(Optional)
import std.iter.(Iterator, Iterable)
import std.collections.(Slice)

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
/// let p = RawPointer.nilPointer();
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
    public static func nilPointer() -> RawPointer {
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
    public func cast[T]() -> Pointer[T] {
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
        hasher.write(ArraySlice(pointer: Pointer(to: x).asRaw().cast[UInt8](), count: 8))
    }
}

/// Typed pointer to a single value of `T`.
///
/// Element-typed counterpart to `RawPointer`: `offset(by:)` strides in
/// units of `sizeof[T]`, and `pointee` reads/writes through the address.
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
public struct Pointer[T]: Equatable, Hashable {
    private var _raw: lang.ptr[T]

    /// The wrapped primitive pointer.
    public var raw: lang.ptr[T] { self._raw }

    /// @name From Raw
    /// Wraps an existing primitive pointer.
    public init(raw raw: lang.ptr[T]) {
        self._raw = raw;
    }

    /// @name To Value
    /// Takes the address of `value`. Equivalent to `&value` in C — the
    /// caller must ensure `value` outlives any use of the resulting
    /// pointer.
    public init(to value: T) {
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
    public var pointee: T {
        get { lang.ptr_read(self._raw) }
        set { lang.ptr_write(self._raw, newValue) }
    }

    /// Numeric address — same value as `asRaw().address`.
    public var address: UInt64 {
        UInt64(intLiteral: lang.ptr_to_address(lang.cast_ptr[_, lang.i8](self._raw)))
    }

    /// Convenience for `address == 0`.
    public var isNull: Bool {
        Bool(boolLiteral: lang.ptr_is_null(lang.cast_ptr[_, lang.i8](self._raw)))
    }

    /// Reads `T` from the address. Same safety preconditions as `pointee.get`.
    public func read() -> T {
        lang.ptr_read(self._raw)
    }

    /// Writes `value` through the pointer. Same safety preconditions as
    /// `pointee.set`.
    public func write(value: T) {
        lang.ptr_write(self._raw, value)
    }

    /// Strides the pointer by `n` *elements* (multiplied by `sizeof[T]`).
    /// Compare with `RawPointer.offset`, which strides by raw bytes.
    public func offset(by n: Int64) -> Pointer[T] {
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
    public func cast[U]() -> Pointer[U] {
        Pointer(raw: lang.cast_ptr[_, U](lang.cast_ptr[_, lang.i8](self._raw)))
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
        hasher.write(ArraySlice(pointer: Pointer(to: x).asRaw().cast[UInt8](), count: 8))
    }
}

/// `Pointer[T]` is FFI-safe whenever its element type is.
extend Pointer: FFISafe where T: FFISafe {}

/// Non-owning view over a contiguous run of `T` values.
///
/// `Slice` is the standard "borrow" type for arrays, buffers, and any
/// other contiguous storage: it stores a pointer + length and provides
/// safe and unchecked indexing, sub-slicing, iteration, and pattern
/// matching. The slice does **not** track or extend the lifetime of the
/// underlying storage — keeping a slice past the end of its source is a
/// use-after-free.
///
/// # Examples
///
/// ```
/// let arr = [1, 2, 3, 4];
/// let s = arr.asSlice();
/// s[safe: 0]                    // .Some(1)
/// s[safe: 99]                   // .None
/// for x in s.iter() { print(x) }
/// ```
///
/// # Memory Model
///
/// Non-owning. Drop the source (`Array`, `Buffer`, literal scope) and the
/// slice becomes dangling. Slices freely copy — they're just `(ptr, len)`
/// pairs.
@builtin(.SliceStruct)
public struct ArraySlice[T] {
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

    /// @name Indexed
    /// Reads or writes at `index`, panicking on out-of-bounds.
    ///
    /// Generic over `SliceIndex[T]`: `Int64` reads/writes a single
    /// element, `Range[Int64]` and `ClosedRange[Int64]` read or replace
    /// a sub-slice. Range writes require the source slice's length to
    /// match the range's length and panic otherwise. Sub-slices alias
    /// the receiver's storage; don't outlive it.
    public subscript[I](index: I) -> I.SliceYield where I: SliceIndex[T] {
        get { index.readSlice(from: self) }
        set { index.writeSlice(to: self, with: newValue) }
    }

    /// @name Checked Index
    /// Reads at `index`, returning `.None` on out-of-bounds.
    public subscript[I](checked index: I) -> I.SliceYield? where I: SliceIndex[T] {
        get { index.readSliceChecked(from: self) }
    }

    /// @name Unchecked Index
    /// Reads or writes at `index` without a bounds check.
    ///
    /// # Safety
    ///
    /// Undefined behavior if the access falls outside `[0, count)`.
    public subscript[I](unchecked index: I) -> I.SliceYield where I: SliceIndex[T] {
        get { index.readSliceUnchecked(from: self) }
        set { index.writeSliceUnchecked(to: self, with: newValue) }
    }

    /// @name Clamping
    /// Reads or writes at `index` with bounds saturated to `[0, count)`.
    /// `Int64` yields `T?` (`None` on empty slice); range indexes yield
    /// `ArraySlice[T]`.
    public subscript[I](clamped index: I) -> I.SliceClampedYield where I: SliceClampable[T] {
        get { index.readSliceClamped(from: self) }
        set { index.writeSliceClamped(to: self, with: newValue) }
    }

    /// @name Wrapping
    /// Reads or writes at `index` using modulo-wrapping. Yields `T?` so
    /// the empty-slice case can surface as `None`.
    public subscript[I](wrapped index: I) -> I.SliceWrappedYield where I: SliceWrappable[T] {
        get { index.readSliceWrapped(from: self) }
        set { index.writeSliceWrapped(to: self, with: newValue) }
    }

    /// Forward iterator over the elements.
    public func iter() -> ArraySliceIterator[T] {
        ArraySliceIterator(ptr: self.ptr, remaining: self.len)
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

/// `ArrayMatchable` conformance so `Slice` can appear in array patterns
/// (`[a, b]`, `[a, ..rest]`, `[a, .., z]`). The `match*` methods skip
/// bounds checks because the compiler has already verified them.
extend ArraySlice[T]: ArrayMatchable {
    type Element = T

    /// Element count, exposed to the pattern matcher.
    public func matchLength() -> Int64 {
        self.count
    }

    /// Compiler-driven element read; safe to skip the bounds check
    /// because the matcher emits `index < matchLength()` first.
    public func matchGet(index: Int64) -> T {
        self.pointer.offset(by: index).read()
    }

    /// Sub-slice for rest-pattern bindings (`..rest`). As above, the
    /// matcher guarantees `0 <= from <= to <= matchLength()`.
    public func matchSlice(from: Int64, to: Int64) -> ArraySlice[T] {
        ArraySlice(pointer: self.pointer.offset(by: from), count: to - from)
    }
}

/// `Slice[T]` conformance — `ArraySlice` is the kernel type, so
/// `asSlice()` returns `self`. Also declares the `Iterable` associated
/// types so slices can be used in `for`-`in` loops and generic
/// `I: Iterable` contexts.
extend ArraySlice[T]: Slice[T], Iterable {
    type Item = T
    type TargetIterator = ArraySliceIterator[T]

    /// Returns `self` — `ArraySlice` is already the borrowed view.
    public func asSlice() -> ArraySlice[T] { self }
}

/// Element-wise equality when the element type is `Equatable`.
///
/// # Examples
///
/// ```
/// let a = [1, 2, 3].asSlice();
/// let b = [1, 2, 3].asSlice();
/// a == b;  // true
///
/// let c = [4, 5, 6].asSlice();
/// a == c;  // false (same length, different elements)
/// ```
extend ArraySlice[T]: Equatable where T: Equatable {
    /// Compares element-by-element. Short-circuits on the first mismatch.
    public func isEqual(to other: ArraySlice[T]) -> Bool {
        if self.len != other.len {
            return false
        }
        for i in 0..<self.len {
            if self.ptr.offset(by: i).read().isEqual(to: other.ptr.offset(by: i).read()) == false {
                return false
            }
        }
        true
    }
}

/// Forward iterator over an `ArraySlice[T]`. Holds a moving pointer and a
/// remaining count; advancing reads through the pointer.
///
/// # Representation
///
/// A `Pointer[T]` cursor and an `Int64` countdown.
public struct ArraySliceIterator[T]: Iterator {
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

// ============================================================================
// SLICE INDEX PROTOCOLS
// ============================================================================
//
// Mirror of `ArrayIndex` / `ArrayClampable` / `ArrayWrappable` but scoped
// to `ArraySlice[T]`. Slice has no COW barrier, so writes go directly through
// the pointer — no `mutating` on the slice parameter. Sealed (internal)
// so user code can't add new index types.

internal protocol SliceIndex[T] {
    type SliceYield
    func readSlice(from slice: ArraySlice[T]) -> SliceYield
    func readSliceChecked(from slice: ArraySlice[T]) -> SliceYield?
    func readSliceUnchecked(from slice: ArraySlice[T]) -> SliceYield
    func writeSlice(to slice: ArraySlice[T], with value: SliceYield)
    func writeSliceUnchecked(to slice: ArraySlice[T], with value: SliceYield)
}

internal protocol SliceClampable[T] {
    type SliceClampedYield
    func readSliceClamped(from slice: ArraySlice[T]) -> SliceClampedYield
    func writeSliceClamped(to slice: ArraySlice[T], with value: SliceClampedYield)
}

internal protocol SliceWrappable[T] {
    type SliceWrappedYield
    func readSliceWrapped(from slice: ArraySlice[T]) -> SliceWrappedYield
    func writeSliceWrapped(to slice: ArraySlice[T], with value: SliceWrappedYield)
}

// ============================================================================
// SLICE INDEX CONFORMANCES
// ============================================================================

extend Int64: SliceIndex[T] {
    type SliceYield = T

    public func readSlice(from slice: ArraySlice[T]) -> T {
        if self < 0 or self >= slice.count {
            fatalError("Slice index out of bounds")
        }
        slice.pointer.offset(by: self).read()
    }

    public func readSliceChecked(from slice: ArraySlice[T]) -> T? {
        if self >= 0 and self < slice.count {
            .Some(slice.pointer.offset(by: self).read())
        } else {
            .None
        }
    }

    public func readSliceUnchecked(from slice: ArraySlice[T]) -> T {
        slice.pointer.offset(by: self).read()
    }

    public func writeSlice(to slice: ArraySlice[T], with value: T) {
        if self < 0 or self >= slice.count {
            fatalError("Slice index out of bounds")
        }
        slice.pointer.offset(by: self).write(value)
    }

    public func writeSliceUnchecked(to slice: ArraySlice[T], with value: T) {
        slice.pointer.offset(by: self).write(value)
    }
}

extend Int64: SliceClampable[T] {
    type SliceClampedYield = T?

    public func readSliceClamped(from slice: ArraySlice[T]) -> T? {
        let len = slice.count;
        if len == 0 {
            return .None
        }
        var idx = self;
        if idx < 0 { idx = 0 }
        if idx >= len { idx = len - 1 }
        .Some(slice.pointer.offset(by: idx).read())
    }

    public func writeSliceClamped(to slice: ArraySlice[T], with value: T?) {
        if let .Some(v) = value {
            let len = slice.count;
            if len == 0 {
                return
            }
            var idx = self;
            if idx < 0 { idx = 0 }
            if idx >= len { idx = len - 1 }
            slice.pointer.offset(by: idx).write(v)
        }
    }
}

extend Int64: SliceWrappable[T] {
    type SliceWrappedYield = T?

    public func readSliceWrapped(from slice: ArraySlice[T]) -> T? {
        let len = slice.count;
        if len == 0 {
            return .None
        }
        var idx = self % len;
        if idx < 0 { idx = idx + len }
        .Some(slice.pointer.offset(by: idx).read())
    }

    public func writeSliceWrapped(to slice: ArraySlice[T], with value: T?) {
        if let .Some(v) = value {
            let len = slice.count;
            if len == 0 {
                return
            }
            var idx = self % len;
            if idx < 0 { idx = idx + len }
            slice.pointer.offset(by: idx).write(v)
        }
    }
}

extend Range[Int64]: SliceIndex[T] {
    type SliceYield = ArraySlice[T]

    public func readSlice(from slice: ArraySlice[T]) -> ArraySlice[T] {
        let start = self.start;
        let end = self.end;
        if start < 0 or end > slice.count or start > end {
            fatalError("Slice range out of bounds")
        }
        ArraySlice(pointer: slice.pointer.offset(by: start), count: end - start)
    }

    public func readSliceChecked(from slice: ArraySlice[T]) -> ArraySlice[T]? {
        let start = self.start;
        let end = self.end;
        if start >= 0 and end <= slice.count and start <= end {
            .Some(ArraySlice(pointer: slice.pointer.offset(by: start), count: end - start))
        } else {
            .None
        }
    }

    public func readSliceUnchecked(from slice: ArraySlice[T]) -> ArraySlice[T] {
        ArraySlice(pointer: slice.pointer.offset(by: self.start), count: self.end - self.start)
    }

    public func writeSlice(to slice: ArraySlice[T], with value: ArraySlice[T]) {
        let start = self.start;
        let end = self.end;
        if start < 0 or end > slice.count or start > end {
            fatalError("Slice range out of bounds")
        }
        let rangeLen = end - start;
        if value.count != rangeLen {
            fatalError("Slice length doesn't match range length")
        }
        var i = 0;
        while i < rangeLen {
            slice.pointer.offset(by: start + i).write(value.pointer.offset(by: i).read());
            i = i + 1;
        }
    }

    public func writeSliceUnchecked(to slice: ArraySlice[T], with value: ArraySlice[T]) {
        let start = self.start;
        let rangeLen = self.end - start;
        if value.count != rangeLen {
            fatalError("Slice length doesn't match range length")
        }
        var i = 0;
        while i < rangeLen {
            slice.pointer.offset(by: start + i).write(value.pointer.offset(by: i).read());
            i = i + 1;
        }
    }
}

extend Range[Int64]: SliceClampable[T] {
    type SliceClampedYield = ArraySlice[T]

    public func readSliceClamped(from slice: ArraySlice[T]) -> ArraySlice[T] {
        let len = slice.count;
        var start = self.start;
        var end = self.end;
        if start < 0 { start = 0 }
        if end > len { end = len }
        if start > end { start = end }
        ArraySlice(pointer: slice.pointer.offset(by: start), count: end - start)
    }

    public func writeSliceClamped(to slice: ArraySlice[T], with value: ArraySlice[T]) {
        let len = slice.count;
        var start = self.start;
        var end = self.end;
        if start < 0 { start = 0 }
        if end > len { end = len }
        if start > end { start = end }
        let rangeLen = end - start;
        if value.count != rangeLen {
            fatalError("Slice length doesn't match clamped range length")
        }
        var i = 0;
        while i < rangeLen {
            slice.pointer.offset(by: start + i).write(value.pointer.offset(by: i).read());
            i = i + 1;
        }
    }
}

extend ClosedRange[Int64]: SliceIndex[T] {
    type SliceYield = ArraySlice[T]

    public func readSlice(from slice: ArraySlice[T]) -> ArraySlice[T] {
        let start = self.start;
        let endExclusive = self.end + 1;
        if start < 0 or endExclusive > slice.count or start > endExclusive {
            fatalError("Slice range out of bounds")
        }
        ArraySlice(pointer: slice.pointer.offset(by: start), count: endExclusive - start)
    }

    public func readSliceChecked(from slice: ArraySlice[T]) -> ArraySlice[T]? {
        let start = self.start;
        let endExclusive = self.end + 1;
        if start >= 0 and endExclusive <= slice.count and start <= endExclusive {
            .Some(ArraySlice(pointer: slice.pointer.offset(by: start), count: endExclusive - start))
        } else {
            .None
        }
    }

    public func readSliceUnchecked(from slice: ArraySlice[T]) -> ArraySlice[T] {
        let start = self.start;
        let endExclusive = self.end + 1;
        ArraySlice(pointer: slice.pointer.offset(by: start), count: endExclusive - start)
    }

    public func writeSlice(to slice: ArraySlice[T], with value: ArraySlice[T]) {
        let start = self.start;
        let endExclusive = self.end + 1;
        if start < 0 or endExclusive > slice.count or start > endExclusive {
            fatalError("Slice range out of bounds")
        }
        let rangeLen = endExclusive - start;
        if value.count != rangeLen {
            fatalError("Slice length doesn't match range length")
        }
        var i = 0;
        while i < rangeLen {
            slice.pointer.offset(by: start + i).write(value.pointer.offset(by: i).read());
            i = i + 1;
        }
    }

    public func writeSliceUnchecked(to slice: ArraySlice[T], with value: ArraySlice[T]) {
        let start = self.start;
        let rangeLen = self.end + 1 - start;
        if value.count != rangeLen {
            fatalError("Slice length doesn't match range length")
        }
        var i = 0;
        while i < rangeLen {
            slice.pointer.offset(by: start + i).write(value.pointer.offset(by: i).read());
            i = i + 1;
        }
    }
}
