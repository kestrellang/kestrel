// Array[T] - dynamic growable array with COW (Copy-on-Write) semantics

module std.collections

import std.core.(Bool, Equatable, Comparable, Ordering, Cloneable, ArrayMatchable, Defaultable, fatalError)
import std.core.(ExpressibleByArrayLiteral, _ExpressibleByArrayLiteral)
import std.core.(Range, ClosedRange, Hashable)
import std.collections.(SeqRange)
import std.text.(Formattable, FormatOptions, StringBuilder)
import std.numeric.(Int64, UInt8)
import std.numeric.(RandomNumberGenerator, Lcg64, DefaultRandom)
import std.result.(Optional)
import std.memory.(Layout, Pointer, UnsafeSlice, UnsafeSliceIterator, RawPointer, Allocator, GlobalAllocator, LiteralSlice, CowBox)
import std.ffi.(memcpy, memmove)
import std.iter.(Iterator, Iterable)
import std.text.(String)
import std.collections.(Sliceable, Slice, SliceIndex, SliceClampable)

/// Forward iterator over an `Array[T]`: an `UnsafeSliceIterator[T]` cursor plus
/// a share of the array's storage.
///
/// The share keeps the buffer alive and marks it shared, so the iterator sees
/// the array's value at the moment `iter()` was called (value semantics, as in
/// Swift): appending to or reassigning the array while iterating copies or
/// drops the array's own buffer, never the one being read, and an iterator
/// returned from a function outlives the array it came from.
///
/// # Representation
///
/// A `CowBox` share (one refcount) and the `(ptr, remaining)` cursor.
public struct ArrayIterator[T, A = GlobalAllocator]: Iterator, Cloneable where A: Allocator {
    type Item = T

    private var storage: CowBox[ArrayStorage[T, A]]
    private var cursor: UnsafeSliceIterator[T]

    /// @name From Storage
    /// Holds `storage` for the cursor's lifetime.
    fileprivate init(storage storage: CowBox[ArrayStorage[T, A]], cursor cursor: UnsafeSliceIterator[T]) {
        self.storage = storage;
        self.cursor = cursor;
    }

    /// Yields the next element, or `.None` at the end.
    public mutating func next() -> Optional[T] {
        self.cursor.next()
    }

    /// An independent cursor at the same position, holding its own share.
    public func clone() -> ArrayIterator[T, A] {
        ArrayIterator(storage: self.storage.clone(), cursor: self.cursor)
    }
}

// ChunksIterator and WindowsIterator live in views.ks (alongside
// ChunksView / WindowsView).

// ============================================================================
// ARRAY STORAGE (Internal)
// ============================================================================

/// Internal `(ptr, len, cap)` storage cell shared by `Array[T]` instances.
///
/// Wrapped in an `RcBox` by `Array[T]` so that copying an `Array` simply
/// bumps a reference count; mutations call `makeUnique()` first to perform
/// the actual copy. The `clone()` method here is the deep-copy half of that
/// COW protocol — it allocates a fresh buffer and copies every element.
/// Owners of the buffer are responsible for freeing it; the `deinit`
/// handles that automatically when the last reference drops.
///
/// # Examples
///
/// ```
/// // Not used directly. Created by Array's initializers.
/// let s = ArrayStorage(ptr: ptr, len: 3, cap: 4);
/// ```
///
/// # Representation
///
/// Three fields: a heap pointer to the element buffer, a length (number of
/// initialized elements), and a capacity (allocation size in elements).
/// `cap == 0` indicates a null `ptr` and no allocation.
///
/// # Memory Model
///
/// Owns the heap buffer. Deallocation happens in `deinit`. Used as a value
/// inside `RcBox`, which provides the reference counting that makes COW
/// possible.
struct ArrayStorage[T, A]: Cloneable where A: Allocator {
    /// Heap pointer to the element buffer; null when `cap == 0`.
    var ptr: Pointer[T]
    /// Number of initialized elements stored in the buffer.
    var len: Int64
    /// Total slots allocated; always `>= len`.
    var cap: Int64
    /// Where the buffer came from and goes back to. Copied with the
    /// storage, so an allocator must be a handle (stateless, or pointing
    /// at shared state like an arena).
    var allocator: A

    /// @name From Fields
    /// Constructs an `ArrayStorage` from raw fields.
    ///
    /// The caller is responsible for guaranteeing the invariants
    /// (`len <= cap`, `ptr` valid for `cap` elements when `cap > 0`).
    ///
    /// # Safety
    ///
    /// Internal: callers must pass consistent values. `Array` controls all
    /// allocation paths.
    ///
    /// # Examples
    ///
    /// ```
    /// let s = ArrayStorage(ptr: rawPtr.cast[T](), len: 0, cap: 16, allocator: SystemAllocator());
    /// ```
    init(ptr ptr: Pointer[T], len len: Int64, cap cap: Int64, allocator allocator: A) {
        self.ptr = ptr;
        self.len = len;
        self.cap = cap;
        self.allocator = allocator;
    }

    /// Deep-copies the storage into a freshly allocated buffer.
    ///
    /// Allocates a new buffer sized exactly to `len` (so the clone has no
    /// excess capacity) and copies each element via `read()` / `write()`.
    /// An empty source returns an empty storage with a null pointer.
    /// Panics if allocation fails. This is the slow half of COW — it runs
    /// when `Array.makeUnique()` detects shared storage on a mutation.
    ///
    /// # Examples
    ///
    /// ```
    /// let copy = storage.clone();
    /// ```
    func clone() -> ArrayStorage[T, A] {
        if self.cap == 0 {
            return ArrayStorage(
                ptr: Pointer[T].nullPointer(),
                len: 0,
                cap: 0,
                allocator: self.allocator
            )
        }
        let layout = Layout.array[T](self.cap);
        var allocator = self.allocator;
        let result = allocator.allocate(layout);
        if let .Some(rawPtr) = result {
            let newPtr = rawPtr.cast[T]();
            for i in 0..<self.len {
                unsafe newPtr.offset(by: i).write(self.ptr.offset(by: i).read());
            }
            ArrayStorage(ptr: newPtr, len: self.len, cap: self.cap, allocator: allocator)
        } else {
            fatalError("ArrayStorage clone allocation failed")
        }
    }

    /// Drops every live element, then frees the underlying buffer.
    ///
    /// Runs when the last `RcBox` reference to this storage drops (COW
    /// guarantees the buffer is uniquely owned at that point, so each
    /// element is dropped exactly once — no double-free). Skips
    /// everything when `cap == 0` (no buffer was ever allocated).
    ///
    /// The `0..<len` loop runs `T`'s destructor in place on each
    /// initialized slot; slots `len..<cap` are uninitialized capacity and
    /// must not be touched. For a trivially-droppable `T` (scalars), the
    /// per-element `dropInPlace` lowers to a no-op. Skipping element
    /// destructors here (the previous behavior) leaked the owned heap of
    /// every non-trivial element, e.g. the `String`s inside an
    /// `Array[(String, String)]`.
    deinit {
        if self.cap > 0 {
            var i: Int64 = 0;
            while i < self.len {
                unsafe self.ptr.offset(by: i).dropInPlace();
                i = i + 1
            };
            let layout = Layout.array[T](self.cap);
            var allocator = self.allocator;
            unsafe allocator.deallocate(self.ptr.asRaw(), layout)
        }
    }
}

// ============================================================================
// ARRAY
// ============================================================================

/// A dynamic, growable, contiguous-buffer array with copy-on-write storage.
///
/// `Array[T]` is the standard ordered-collection type. It supports
/// constant-time random access, amortized constant-time `append`, and
/// arbitrary-position insert/remove via shifting. Storage is shared between
/// copies until one of them mutates, at which point that copy lazily clones
/// the buffer (see "Memory Model" below). For non-owning views over an
/// existing buffer use `UnsafeSlice[T]`; for fixed-size or set-like collections
/// see `UnsafeSlice[T]`, `Set`, or `Dictionary`.
///
/// # Examples
///
/// ```
/// let evens = [2, 4, 6, 8];
/// var names = Array[String]();
/// names.append("Alice");
/// names.append("Bob");
///
/// let copy = names;      // O(1) — shares storage with `names`
/// names.append("Carol"); // O(1) clone happens here, `copy` is unchanged
///
/// for n in names.iter() { ... }
/// let pivot = names.partition(by: { (n) in n.count > 3 });
/// ```
///
/// # Indexing
///
/// The default subscript `arr(i)` panics on out-of-bounds. Variants exist
/// for every common policy, each an in-place place: `arr(checked: i)` is
/// `.None` when out of range, `unsafe arr(unchecked: i)` skips the bounds check
/// (UB on OOB), `arr(wrapped: i)` wraps with modulo (negative indices count
/// from the end), and `arr(clamped: i)` saturates into `[0, count-1]`.
/// Ranges take `arr(0..<3)`, `arr(checked: r)` and `arr(unchecked: r)`,
/// and `arr(within: r)` lends the part of `r` inside the array.
///
/// # Capacity & Reallocation
///
/// `count` is the number of elements; `capacity` is how many can fit
/// without reallocating. When `append` would exceed capacity the buffer
/// doubles (starting from 4 if previously zero). Use
/// `reserveCapacity(_:)` to pre-allocate, and
/// `shrinkToFit()` to release excess.
///
/// # Representation
///
/// Holds a single `CowBox[ArrayStorage[T, A]]` field. The storage is a
/// `(ptr, len, cap)` triple over a heap-allocated buffer.
///
/// # Memory Model
///
/// Reference-counted storage with copy-on-write *value* semantics. Copying
/// an `Array` is O(1) and shares the buffer; the next mutation on a shared
/// `Array` triggers `makeUnique()`, which deep-clones the buffer so the
/// mutation is invisible to other copies. The user-visible behavior is
/// indistinguishable from deep-copying on assignment.
///
/// # Guarantees
///
/// - Elements are stored contiguously and are accessible via `asPointer()`
///   for FFI; the pointer is invalidated by any mutation that may
///   reallocate.
/// - `count <= capacity` always.
/// - Iteration order is insertion order.
/// - Operations marked O(1) are amortized; growth is geometric.
@builtin(.ArrayStruct)
public struct Array[T, A = GlobalAllocator]: Sliceable[T], Iterable, Cloneable where A: Allocator {
    /// `Iterable` element type — the element produced by `iter().next()`.
    type Item = T
    /// `Iterable` iterator type — the concrete iterator returned by `iter()`.
    type TargetIterator = ArrayIterator[T, A]

    /// COW storage — `CowBox` handles the reference counting and
    /// clone-on-write barrier. Sharing this between `Array` copies is
    /// what enables COW.
    fileprivate var storage: CowBox[ArrayStorage[T, A]]

    /// Returns the raw element pointer. Internal helper for storage access.
    fileprivate func ptr() -> Pointer[T] { self.storage.value.ptr }
    /// Returns the element count from the storage. Internal helper.
    fileprivate func len() -> Int64 { self.storage.value.len }
    /// Returns the buffer capacity from the storage. Internal helper.
    fileprivate func cap() -> Int64 { self.storage.value.cap }

    /// COW write barrier — ensures the storage is uniquely owned.
    fileprivate mutating func makeUnique() {
        self.storage.ensureUnique()
    }

    /// @name From Storage
    /// Wraps an existing storage box in a new `Array`.
    ///
    /// Module-internal — used by `clone()`, `ArrayBuilder.build()`, and
    /// other `std.collections` code that constructs arrays from raw
    /// storage.
    init(storage storage: CowBox[ArrayStorage[T, A]]) {
        self.storage = storage;
    }

    // ========================================================================
    // CONSTRUCTORS (explicit allocator)
    // ========================================================================

    /// @name With Allocator
    /// An empty array whose buffer will come from `allocator`. Nothing is
    /// allocated until the first insert.
    ///
    /// # Examples
    ///
    /// ```
    /// var arena = ArenaAllocator();
    /// var xs = Array[Int64, ArenaAllocator](allocator: arena);
    /// xs.append(1);  // allocated from the arena
    /// ```
    public init(allocator allocator: A) {
        self.storage = CowBox(ArrayStorage(ptr: Pointer[T].nullPointer(), len: 0, cap: 0, allocator: allocator));
    }

    /// @name With Capacity And Allocator
    /// An empty array with room for `capacity` elements, allocated from
    /// `allocator`. Panics on allocation failure.
    public init(capacity capacity: Int64, allocator allocator: A) {
        self.init(allocator: allocator);
        self.grow(capacity)
    }

    /// @name Repeating Value With Allocator
    /// `count` copies of `value`, allocated from `allocator`.
    ///
    /// When `value` is copied byte for byte and its bytes are all zero, the
    /// buffer comes from `allocator.allocateZeroed` and nothing is written:
    /// zeroed memory already holds the copies.
    ///
    /// # Examples
    ///
    /// ```
    /// var arena = ArenaAllocator();
    /// let zeros = Array[Int64, ArenaAllocator](repeating: 0, count: 4, allocator: arena);
    /// ```
    public init(repeating value: T, count count: Int64, allocator allocator: A) {
        if count <= 0 {
            self.init(allocator: allocator)
        } else {
            let layout = Layout.array[T](count);
            var alloc = allocator;
            // A byte-copied value whose bytes are all zero: zeroed memory
            // already holds `count` copies of it, and `calloc` hands back a
            // big block without touching its pages.
            let zero = Bool(boolLiteral: lang.is_bitwise[T]()) and
                bytesAreZero(Pointer[T](raw: lang.ptr_to(value)).asRaw(), Layout.of[T]().size);
            let result = if zero { alloc.allocateZeroed(layout) } else { alloc.allocate(layout) };
            if let .Some(rawPtr) = result {
                let newPtr = rawPtr.cast[T]();
                if not zero {
                    for i in 0..<count {
                        unsafe newPtr.offset(by: i).write(value);
                    }
                }
                self.storage = CowBox(ArrayStorage(
                    ptr: newPtr,
                    len: count,
                    cap: count,
                    allocator: alloc
                ))
            } else {
                fatalError("Array allocation failed")
            }
        }
    }

    /// The allocator this array's buffer comes from.
    public var allocator: A { self.storage.value.allocator }

    // ========================================================================
    // PROPERTIES
    // ========================================================================

    // count, isEmpty, indices, asPointer, isValid(index:): provided by extend Sliceable[T]

    /// The number of elements the buffer can hold without reallocating.
    public var capacity: Int64 { self.cap() }

    /// The elements as a slice, in place. Valid until the array next
    /// reallocates or is dropped.
    public func asSlice() -> &Slice[T] {
        unsafe Slice.lend(start: self.ptr(), count: self.len())
    }

    /// The elements as a writable slice, in place. Runs the copy-on-write
    /// check first, so writes never reach a copy's buffer. The slice can
    /// change elements, never the array's length.
    public mutating func asMutableSlice() -> &mutating Slice[T] {
        self.makeUnique();
        unsafe Slice.lendMutating(start: self.ptr(), count: self.len())
    }

    /// The `capacity - count` slots past the last element, for filling in
    /// place (a stream `read` straight into the array, say); `setCount`
    /// then claims the filled ones. Grow first with `reserveCapacity`.
    ///
    /// # Safety
    ///
    /// The slots are uninitialized: nothing may be read from one before
    /// it is written, and only slots that were written may be claimed.
    ///
    /// # Examples
    ///
    /// ```
    /// var bytes = Array[UInt8]();
    /// bytes.reserveCapacity(4096);
    /// let n = try input.read(into: unsafe bytes.spareCapacity());
    /// unsafe bytes.setCount(bytes.count + n);
    /// ```
    public unsafe mutating func spareCapacity() -> &mutating Slice[T] {
        self.makeUnique();
        let myLen = self.len();
        unsafe Slice.lendMutating(start: self.ptr().offset(by: myLen), count: self.cap() - myLen)
    }

    /// Sets `count` without touching any element: claims slots filled
    /// through `spareCapacity`, or forgets trailing ones without dropping
    /// them. Panics when `newCount` is negative or above `capacity`.
    ///
    /// # Safety
    ///
    /// Every slot below `newCount` must hold an initialized element.
    public unsafe mutating func setCount(newCount: Int64) {
        if newCount < 0 or newCount > self.cap() {
            fatalError("Array.setCount: count out of range")
        }
        self.makeUnique();
        self.storage.value.len = newCount
    }

    /// Element access and sub-slicing, in place: `xs(2)` is a reference to
    /// an element (`let x = xs(2)` binds a copy), `xs(1..<3)` a `Slice` of
    /// the array's elements. Writes (`xs(0) = v`, `xs(0) += 1`,
    /// `xs(0..<2).sort()`) run the copy-on-write check first. Panics when
    /// the index or range doesn't fit.
    ///
    /// Copying the array inside the same expression that uses the place
    /// (`f(xs(0), xs)` with a mutating first argument) re-shares the buffer,
    /// so the write can show through the copy: accepted stage-1 behavior,
    /// recorded in the references semantics.
    public subscript[I](index: I) -> I.SliceOutput where I: SliceIndex[T] {
        ref { I.lend(index, self.asSlice()) }
        mutating ref { I.lendMutating(index, self.asMutableSlice()) }
    }

    /// Like `xs(index)`, without the bounds check.
    ///
    /// # Safety
    ///
    /// `index` must be in range. Out of range is undefined behavior.
    public unsafe subscript[I](unchecked index: I) -> I.SliceOutput where I: SliceIndex[T] {
        ref { unsafe I.lendUnchecked(index, self.asSlice()) }
        mutating ref { unsafe I.lendMutatingUnchecked(index, self.asMutableSlice()) }
    }

    /// Like `xs(index)`, but the place may not exist: `.None` instead of a
    /// panic when out of range. Reading copies the element out (`T?`) or
    /// lends the slice; `if let .Some(&mutating v) = xs(checked: i)` writes
    /// in place.
    public subscript[I](checked index: I)? -> I.SliceOutput where I: SliceIndex[T] {
        ref { I.lendChecked(index, self.asSlice()) }
        mutating ref { I.lendMutatingChecked(index, self.asMutableSlice()) }
    }

    /// The element at `index` saturated into `0..<count`, in place, or
    /// `.None` for an empty array.
    public subscript(clamped index: Int64)? -> T {
        ref { self.asSlice().lendClamped(index) }
        mutating ref { self.asMutableSlice().lendMutatingClamped(index) }
    }

    /// The part of `range` that lies within the array, in place (always
    /// valid, possibly empty).
    public subscript[R](within range: R) -> Slice[T] where R: SliceClampable[T] {
        ref { R.lendWithin(range, self.asSlice()) }
        mutating ref { R.lendMutatingWithin(range, self.asMutableSlice()) }
    }

    /// The element at `index` modulo `count` (negative indices count from
    /// the end), in place, or `.None` for an empty array.
    public subscript(wrapped index: Int64)? -> T {
        ref { self.asSlice().lendWrapped(index) }
        mutating ref { self.asMutableSlice().lendMutatingWrapped(index) }
    }

    /// COW write barrier — deep-copies storage if shared.
    public mutating func ensureUnique() {
        self.makeUnique()
    }

    // ========================================================================
    // CAPACITY MANAGEMENT (Internal)
    // ========================================================================

    /// Grows the buffer so it can hold at least `minCapacity` elements.
    ///
    /// No-op when current capacity is already sufficient. Otherwise picks
    /// the next capacity by doubling (starting from 4 when capacity is
    /// zero), allocates the new buffer, copies elements over, and frees
    /// the old buffer. Triggers COW first so the reallocation is
    /// invisible to other `Array` copies. Panics if allocation fails.
    // Moves every element into the fresh buffer `newPtr` (capacity `cap`)
    // and frees the old one. The storage must be unique.
    private mutating func moveStorage(to newPtr: Pointer[T], cap cap: Int64) {
        let s = &mutating self.storage.value;
        // A move is a bit copy, so one memcpy moves every element.
        if s.len > 0 {
            let _ = unsafe memcpy(newPtr.asRaw(), s.ptr.asRaw(), s.len * Int64(intLiteral: lang.sizeof[T]()));
        }
        if s.cap > 0 {
            var allocator = s.allocator;
            unsafe allocator.deallocate(s.ptr.asRaw(), Layout.array[T](s.cap))
        }
        s.ptr = newPtr;
        s.cap = cap
    }

    // Bit-moves `n` elements from slot `src` to slot `dst`; the ranges may
    // overlap. Valid for any `T` (a move is a bit copy). The vacated slots
    // are left logically uninitialized, and the storage must be unique.
    private mutating func shiftElements(from src: Int64, to dst: Int64, count n: Int64) {
        if n <= 0 or src == dst {
            return
        }
        let p = self.ptr();
        let _ = unsafe memmove(p.offset(by: dst).asRaw(), p.offset(by: src).asRaw(), n * Int64(intLiteral: lang.sizeof[T]()));
    }

    private mutating func grow(minCapacity: Int64) {
        let myCap = self.cap();
        if myCap >= minCapacity {
            return
        }

        // Calculate new capacity
        var newCap: Int64 = myCap;
        if newCap == 0 {
            newCap = 4
        }
        while newCap < minCapacity {
            newCap = newCap * 2
        }

        let newLayout = Layout.array[T](newCap);
        var allocator = self.storage.value.allocator;
        if self.storage.isUnique and myCap > 0 {
            // Unique: let the allocator grow the block, in place when it
            // can (`realloc`, or an arena's newest block). A move is a bit
            // copy, so the bytes it carries over are valid for any `T`.
            let s = &mutating self.storage.value;
            let grown = unsafe allocator.reallocate(s.ptr.asRaw(), Layout.array[T](myCap), newLayout);
            if let .Some(rawPtr) = grown {
                s.ptr = rawPtr.cast[T]();
                s.cap = newCap;
                return
            }
            fatalError("Array grow failed")
        }
        let result = allocator.allocate(newLayout);
        let newPtr = if let .Some(rawPtr) = result {
            rawPtr.cast[T]()
        } else {
            fatalError("Array grow failed")
        };
        if self.storage.isUnique {
            self.moveStorage(to: newPtr, cap: newCap);
            return
        }
        // Shared: copy straight into the bigger buffer rather than forking
        // an exact-size copy first and then moving it again.
        let src = self.ptr();
        let n = self.len();
        for i in 0..<n {
            unsafe newPtr.offset(by: i).write(src.offset(by: i).read())
        }
        self.storage = CowBox(ArrayStorage(ptr: newPtr, len: n, cap: newCap, allocator: allocator))
    }

    // ========================================================================
    // ELEMENT ACCESS
    // ========================================================================

    // ========================================================================
    // ADDING ELEMENTS
    // ========================================================================

    /// Appends `element` to the end of the array.
    ///
    /// Amortized O(1). Triggers a reallocation (and COW if storage is
    /// shared) when `count == capacity`. For appending many elements,
    /// `reserveCapacity(...)` first to avoid intermediate growths; for
    /// adding multiple elements at once see `append(contentsOf:)` or
    /// `append(from:)`.
    ///
    /// # Examples
    ///
    /// ```
    /// var arr = [1, 2];
    /// arr.append(3);  // [1, 2, 3]
    /// ```
    @inline(.Always)
    public mutating func append(consuming element: T) {
        // Fast path inline: unique storage with a free slot. Growth and
        // the copy-on-write fork stay out of line so callers' loops stay
        // small.
        if not self.storage.isUnique or self.len() == self.cap() {
            self.makeRoomForAppend()
        }
        // Reserve the slot and bump `len` in place, then move the element in.
        let s = &mutating self.storage.value;
        let slot = s.ptr.offset(by: s.len);
        s.len = s.len + 1;
        unsafe slot.write(element)
    }

    /// Slow path of `append`: grows a full buffer (copying a shared one
    /// straight into the bigger buffer), else forks shared storage.
    @inline(.Never)
    private mutating func makeRoomForAppend() {
        self.grow(self.len() + 1);
        self.makeUnique()
    }

    /// Appends every element of `other` to the end of this array.
    ///
    /// Reserves the exact required capacity in one growth step then
    /// copies the elements over, so it's faster than calling `append`
    /// in a loop. Sharing semantics: `other` is read-only here, but if
    /// `self` shares storage with anything else, COW fires once at the
    /// start. See also `append(from:)` for arbitrary iterable
    /// sources.
    ///
    /// # Examples
    ///
    /// ```
    /// var arr = [1, 2];
    /// arr.append(contentsOf: [3, 4]);  // [1, 2, 3, 4]
    /// arr.append(contentsOf: []);      // [1, 2, 3, 4]  — no-op
    /// ```
    public mutating func append(contentsOf other: &some Sliceable[T] and not Static and not Copyable and not Sized) {
        let sl = other.asUnsafeSlice();
        let otherLen = sl.count;
        if otherLen == 0 {
            return
        }
        let myLen = self.len();
        self.grow(myLen + otherLen);
        let otherPtr = sl.asPointer();
        let s = &mutating self.storage.value;
        for i in 0..<otherLen {
            unsafe s.ptr.offset(by: s.len).write(otherPtr.offset(by: i).read());
            s.len = s.len + 1
        }
    }

    /// Appends every element produced by an arbitrary iterable.
    ///
    /// Drains the iterable via `append`, so capacity grows geometrically
    /// rather than to an exact target — for sized sources like another
    /// `Array`, prefer `append(contentsOf:)`.
    ///
    /// # Examples
    ///
    /// ```
    /// var arr = [1, 2];
    /// arr.append(from: 3..<6);  // [1, 2, 3, 4, 5]
    /// ```
    public mutating func append[I](from iterable: I) where I: Iterable, I: not Static, I.Item = T {
        var iter = iterable.iter();
        while let .Some(item) = iter.next() {
            self.append(item)
        }
    }

    /// Inserts `element` at `index`, shifting later elements right by one.
    ///
    /// O(n) in the number of elements after `index`. `index == count`
    /// behaves like `append`. Triggers COW and may reallocate. For bulk
    /// insertion at one location, prefer
    /// `replaceSubrange(i..<i, with: ...)`.
    ///
    /// # Errors
    ///
    /// Panics with `"Array.insert: index out of bounds"` if `index < 0`
    /// or `index > count`.
    ///
    /// # Examples
    ///
    /// ```
    /// var arr = [1, 3];
    /// arr.insert(2, at: 1);  // [1, 2, 3]
    /// arr.insert(0, at: 0);  // [0, 1, 2, 3]
    /// arr.insert(4, at: 4);  // [0, 1, 2, 3, 4]  — append-equivalent
    /// arr.insert(9, at: 99); // PANIC
    /// ```
    public mutating func insert(element: T, at index: Int64) {
        let myLen = self.len();
        if index < 0 or index > myLen {
            fatalError("Array.insert: index out of bounds")
        }
        self.grow(myLen + 1);
        self.makeUnique();
        self.shiftElements(from: index, to: index + 1, count: myLen - index);
        let s = &mutating self.storage.value;
        unsafe s.ptr.offset(by: index).write(element);
        s.len = s.len + 1
    }

    // ========================================================================
    // REMOVING ELEMENTS
    // ========================================================================

    /// Removes and returns the last element, or `None` if the array is empty.
    ///
    /// O(1). Capacity is retained for reuse — only `len` is decremented.
    /// The mirror operation `popFirst()` is O(n) because it must shift
    /// the remainder. To inspect the last element without removing, use
    /// `last()`.
    ///
    /// # Examples
    ///
    /// ```
    /// var arr = [1, 2, 3];
    /// arr.pop();  // Some(3), arr is [1, 2]
    /// arr.pop();  // Some(2), arr is [1]
    /// arr.pop();  // Some(1), arr is []
    /// arr.pop();  // None,    arr is still []
    /// ```
    public mutating func pop() -> T? {
        let myLen = self.len();
        if myLen > 0 {
            self.makeUnique();
            let s = &mutating self.storage.value;
            s.len = s.len - 1;
            // Move out: the slot leaves the live range, so a copy leaked.
            let value = unsafe s.ptr.offset(by: s.len).take();
            .Some(value)
        } else {
            .None
        }
    }

    /// Removes and returns the first element, or `None` if the array is
    /// empty.
    ///
    /// O(n) — every following element shifts left by one. If you can
    /// tolerate it, `pop()` from the back is O(1). For inspection
    /// without removal, use `first()`.
    ///
    /// # Examples
    ///
    /// ```
    /// var arr = [1, 2, 3];
    /// arr.popFirst();  // Some(1), arr is [2, 3]
    /// arr.popFirst();  // Some(2), arr is [3]
    /// ```
    public mutating func popFirst() -> T? {
        if self.len() == 0 {
            return .None
        }
        .Some(self.remove(at: 0))
    }

    /// Removes and returns the element at `index`, shifting later
    /// elements left.
    ///
    /// O(n - index). Capacity is retained. For removing many elements at
    /// once, prefer `removeSubrange(range:)`. To remove the *first*
    /// element by *value* see the `Equatable` extension's
    /// `remove(element:)`.
    ///
    /// # Errors
    ///
    /// Panics with `"Array.remove: index out of bounds"` if `index < 0`
    /// or `index >= count`.
    ///
    /// # Examples
    ///
    /// ```
    /// var arr = [1, 2, 3, 4];
    /// arr.remove(at: 1);  // returns 2; arr is [1, 3, 4]
    /// arr.remove(at: 9);  // PANIC
    /// ```
    public mutating func remove(at index: Int64) -> T {
        let myLen = self.len();
        if index < 0 or index >= myLen {
            fatalError("Array.remove: index out of bounds")
        }
        self.makeUnique();
        // Move (`take`), not `read()`: the slot is overwritten by the
        // shift, which drops nothing, so a copy here leaked.
        let removed = unsafe self.ptr().offset(by: index).take();
        self.shiftElements(from: index + 1, to: index, count: myLen - index - 1);
        self.storage.value.len = myLen - 1;
        removed
    }

    /// Removes every element in `range`, shifting later elements left.
    ///
    /// O(count - range.end + range.length). Empty ranges are no-ops.
    /// Capacity is retained — call `shrinkToFit()` to release it. For
    /// "remove these and put others back" use `replaceSubrange(...)`.
    ///
    /// # Errors
    ///
    /// Panics with `"Array.removeSubrange: range out of bounds"` if
    /// `range.start < 0`, `range.end > count`, or
    /// `range.start > range.end`.
    ///
    /// # Examples
    ///
    /// ```
    /// var arr = [1, 2, 3, 4, 5];
    /// arr.removeSubrange(1..<4);  // arr is [1, 5]
    /// arr.removeSubrange(0..<0);  // no-op
    /// ```
    public mutating func removeSubrange[R](range: R) where R: SeqRange {
        let resolved = range.resolve(self.count);
        let start = resolved.start;
        let end = resolved.end;
        let myLen = self.len();
        if start < 0 or end > myLen or start > end {
            fatalError("Array.removeSubrange: range out of bounds")
        }
        let removeCount = end - start;
        if removeCount == 0 {
            return
        }
        self.makeUnique();
        // Drop the removed elements, then move the tail left.
        let p = self.ptr();
        for k in start..<end {
            unsafe p.offset(by: k).dropInPlace()
        }
        self.shiftElements(from: end, to: start, count: myLen - end);
        self.storage.value.len = myLen - removeCount
    }


    /// Removes every element from the array, leaving capacity untouched.
    ///
    /// O(1). The buffer is kept so subsequent appends don't reallocate
    /// — if you want the memory back, follow with `shrinkToFit()`.
    ///
    /// # Examples
    ///
    /// ```
    /// var arr = [1, 2, 3];
    /// arr.clear();    // arr is []
    /// arr.capacity;   // unchanged
    /// ```
    public mutating func clear() {
        self.makeUnique();
        // Drop every live element (mirrors ArrayStorage.deinit) before
        // resetting len; without this, elements with non-trivial deinits
        // (e.g. heap-owning types) would leak because deinit only drops 0..<len.
        let s = &mutating self.storage.value;
        var i: Int64 = 0;
        while i < s.len {
            unsafe s.ptr.offset(by: i).dropInPlace();
            i = i + 1
        };
        s.len = 0
    }

    /// Keeps only elements for which `predicate` returns true; removes
    /// the rest in place.
    ///
    /// O(n), single pass, stable (relative order preserved). The mirror
    /// operation is `removeAll(where:)`. For a copy instead of an
    /// in-place edit, use `iter().filter(...).collect()`.
    ///
    /// # Examples
    ///
    /// ```
    /// var arr = [1, 2, 3, 4, 5];
    /// arr.retain(where: { (x) in x % 2 == 0 });  // [2, 4]
    /// ```
    public mutating func retain(where predicate: (T) -> Bool) {
        self.makeUnique();
        let s = &mutating self.storage.value;
        var writeIdx: Int64 = 0;
        for readIdx in 0..<s.len {
            let keep = predicate(unsafe s.ptr.offset(by: readIdx).value);
            if keep {
                if writeIdx != readIdx {
                    let element = unsafe s.ptr.offset(by: readIdx).take();
                    unsafe s.ptr.offset(by: writeIdx).write(element)
                }
                writeIdx = writeIdx + 1
            } else {
                unsafe s.ptr.offset(by: readIdx).dropInPlace()
            }
        }
        s.len = writeIdx
    }

    /// Removes every element for which `predicate` returns true.
    ///
    /// The inverse of `retain(where:)` — implemented as
    /// `retain` over the negated predicate. O(n), stable.
    ///
    /// # Examples
    ///
    /// ```
    /// var arr = [1, 2, 3, 4, 5];
    /// arr.removeAll(where: { (x) in x % 2 == 0 });  // [1, 3, 5]
    ///
    /// var names = ["Alice", "", "Bob", ""];
    /// names.removeAll(where: { (s) in s.isEmpty });  // ["Alice", "Bob"]
    /// ```
    public mutating func removeAll(consuming where predicate: (T) -> Bool) {
        self.retain(where: { (x) in predicate(x) == false })
    }

    // ========================================================================
    // REORDERING
    // ========================================================================

    /// Swaps the elements at indices `i` and `j` in place.
    ///
    /// O(1). A no-op when `i == j`. Triggers COW.
    ///
    /// # Errors
    ///
    /// Panics with `"Array.swap: index out of bounds"` if either index
    /// is `< 0` or `>= count`.
    ///
    /// # Examples
    ///
    /// ```
    /// var arr = [1, 2, 3];
    /// arr.swap(at: 0, with: 2);  // [3, 2, 1]
    /// arr.swap(at: 1, with: 1);  // [3, 2, 1] — no-op
    /// arr.swap(at: 0, with: 9);  // PANIC
    /// ```
    public mutating func swap(at i: Int64, with j: Int64) {
        let myLen = self.len();
        if i < 0 or i >= myLen or j < 0 or j >= myLen {
            fatalError("Array.swap: index out of bounds")
        }
        if i == j {
            return
        }
        self.makeUnique();
        let ptr = self.ptr();
        // Move, not `read()`: `write` drops nothing, so a copy leaked.
        let temp = unsafe ptr.offset(by: i).take();
        unsafe ptr.offset(by: i).write(ptr.offset(by: j).take());
        unsafe ptr.offset(by: j).write(temp)
    }

    /// Reverses the order of elements in place.
    ///
    /// O(n). Triggers COW. For a non-mutating variant returning a new
    /// array, use `reversed()`.
    ///
    /// # Examples
    ///
    /// ```
    /// var arr = [1, 2, 3];
    /// arr.reverse();  // [3, 2, 1]
    /// ```
    public mutating func reverse() {
        self.asMutableSlice().reverse()
    }

    // reversed(): provided by extend Sliceable[T] — returns a lazy ReversedView[T]
    // (use `arr.reversed().toArray()` if an owned Array is needed).

    /// Rotates the elements in place by `amount` positions to the left.
    ///
    /// Implemented with the three-reversal algorithm — O(n) time,
    /// O(1) extra space. Negative `amount` rotates right; the actual
    /// rotation is `amount mod count`, so very large amounts wrap. A
    /// no-op when `count <= 1` or the normalized amount is zero.
    ///
    /// # Examples
    ///
    /// ```
    /// var arr = [1, 2, 3, 4, 5];
    /// arr.rotate(by:  2);  // [3, 4, 5, 1, 2]
    /// arr.rotate(by: -1);  // [2, 3, 4, 5, 1]
    /// arr.rotate(by:  7);  // same as rotate(by: 2) for count == 5
    /// ```
    public mutating func rotate(by amount: Int64) {
        self.asMutableSlice().rotate(by: amount)
    }

    /// Replaces the elements in `range` with the elements of `replacement`.
    ///
    /// `replacement.count` need not equal the range length — the array
    /// shrinks or grows accordingly, shifting the trailing elements once.
    /// Use `range == i..<i` to insert without removing, or
    /// `replacement == []` to remove without inserting (equivalent to
    /// `removeSubrange(...)`). May reallocate; triggers COW.
    ///
    /// # Errors
    ///
    /// Panics with `"Array.replaceSubrange: range out of bounds"` if
    /// `range.start < 0`, `range.end > count`, or
    /// `range.start > range.end`.
    ///
    /// # Examples
    ///
    /// ```
    /// var arr = [1, 2, 3, 4, 5];
    /// arr.replaceSubrange(1..<4, with: [20, 30]);    // [1, 20, 30, 5]
    /// arr.replaceSubrange(1..<1, with: [9, 9]);      // insert: [1, 9, 9, 20, 30, 5]
    /// arr.replaceSubrange(0..<2, with: Array[Int64]());  // remove: [9, 20, 30, 5]
    /// ```
    public mutating func replaceSubrange[R, B](range: R, with replacement: Array[T, B]) where R: SeqRange, B: Allocator {
        let resolved = range.resolve(self.count);
        let start = resolved.start;
        let end = resolved.end;
        let myLen = self.len();
        if start < 0 or end > myLen or start > end {
            fatalError("Array.replaceSubrange: range out of bounds")
        }

        let removeCount = end - start;
        let insertCount = replacement.count;
        let newLen = myLen - removeCount + insertCount;

        self.grow(newLen);
        self.makeUnique();
        // Drop the replaced elements, then move the tail into place.
        let p = self.ptr();
        for k in start..<end {
            unsafe p.offset(by: k).dropInPlace()
        }
        self.shiftElements(from: end, to: start + insertCount, count: myLen - end);
        let s = &mutating self.storage.value;

        // Copy replacement
        for i in 0..<insertCount {
            unsafe s.ptr.offset(by: start + i).write(replacement(unchecked: i))
        }

        s.len = newLen
    }

    /// Shuffles the array in place using `rng`.
    ///
    /// Uses the Fisher-Yates algorithm — every permutation is equally
    /// likely, given a uniform RNG. Passing the same seeded `rng`
    /// produces a deterministic shuffle, which is the usual reason to
    /// reach for this overload over the no-arg `shuffle()`.
    ///
    /// # Examples
    ///
    /// ```
    /// var arr = [1, 2, 3, 4, 5];
    /// var rng = Lcg64(seed: 42);
    /// arr.shuffle(using: rng);  // deterministic for the seed
    /// ```
    public mutating func shuffle(mutating using rng: some RandomNumberGenerator) {
        self.asMutableSlice().shuffle(using: rng)
    }

    /// Shuffles the array in place using a fresh default RNG.
    ///
    /// Convenience over `shuffle(using:)`. The result is non-deterministic
    /// across calls — pass an explicit `Lcg64(seed: ...)` (or other
    /// `RandomNumberGenerator`) when you need reproducibility.
    ///
    /// # Examples
    ///
    /// ```
    /// var arr = [1, 2, 3, 4, 5];
    /// arr.shuffle();  // e.g. [3, 1, 5, 2, 4]
    /// ```
    public mutating func shuffle() {
        var rng = DefaultRandom();
        self.shuffle(using: rng)
    }

    /// Returns a new array shuffled with `rng`. The original is unchanged.
    ///
    /// The non-mutating mirror of `shuffle(using:)`. Internally clones via
    /// COW (cheap until the next mutation) and shuffles the copy.
    ///
    /// # Examples
    ///
    /// ```
    /// let arr = [1, 2, 3, 4, 5];
    /// var rng = Lcg64(seed: 42);
    /// let result = arr.shuffled(using: rng);
    /// // arr is still [1, 2, 3, 4, 5]
    /// ```
    public func shuffled(mutating using rng: some RandomNumberGenerator) -> Array[T, A] {
        var result = self.clone();
        result.shuffle(using: rng);
        result
    }

    /// Returns a new array shuffled with a default RNG. Original unchanged.
    ///
    /// Convenience over `shuffled(using:)`. Non-deterministic between
    /// calls.
    ///
    /// # Examples
    ///
    /// ```
    /// let arr = [1, 2, 3, 4, 5];
    /// let shuffled = arr.shuffled();  // e.g. [4, 2, 5, 1, 3]
    /// // arr is still [1, 2, 3, 4, 5]
    /// ```
    public func shuffled() -> Array[T, A] {
        var result = self.clone();
        result.shuffle();
        result
    }

    // ========================================================================
    // CAPACITY MANAGEMENT
    // ========================================================================

    /// Reserves enough capacity to hold at least `minimumCapacity` elements.
    ///
    /// A no-op when capacity already suffices. The actual capacity after
    /// the call may exceed the request because growth rounds up via the
    /// doubling policy. Pair with bulk inserts to skip intermediate
    /// reallocations. The opposite operation is `shrinkToFit()`.
    ///
    /// # Examples
    ///
    /// ```
    /// var arr = Array[Int64]();
    /// arr.reserveCapacity(1000);
    /// for i in 0..<1000 {
    ///         arr.append(i);  // no reallocations
    /// }
    /// ```
    public mutating func reserveCapacity(minimumCapacity: Int64) {
        self.grow(minimumCapacity)
    }

    /// Releases unused capacity by reallocating to fit `count` exactly.
    ///
    /// Useful after a bulk removal or when you've finished building a
    /// large array. A no-op when `capacity == count`. For an empty
    /// array, fully deallocates the buffer (capacity drops to 0).
    /// Triggers COW.
    ///
    /// # Examples
    ///
    /// ```
    /// var arr = Array[Int64](capacity: 1000);
    /// arr.append(1);
    /// arr.shrinkToFit();   // capacity reduced to 1
    /// arr.clear();
    /// arr.shrinkToFit();   // capacity reduced to 0, buffer freed
    /// ```
    public mutating func shrinkToFit() {
        let myLen = self.len();
        let myCap = self.cap();
        if myLen == myCap or myLen == 0 {
            if myLen == 0 and myCap > 0 {
                // Deallocate entirely for empty array
                self.makeUnique();
                let s = &mutating self.storage.value;
                let layout = Layout.array[T](myCap);
                var allocator = s.allocator;
                unsafe allocator.deallocate(s.ptr.asRaw(), layout);
                s.ptr = Pointer[T].nullPointer();
                s.cap = 0
            }
            return
        }

        self.makeUnique();

        // Reallocate to exact size
        let newLayout = Layout.array[T](myLen);
        var allocator = self.storage.value.allocator;
        let result = allocator.allocate(newLayout);
        if let .Some(rawPtr) = result {
            self.moveStorage(to: rawPtr.cast[T](), cap: myLen)
        }
    }

    // first(), last(), firstIndex(where:), lastIndex(where:),
    // first(where:), last(where:), all(where:), any(where:),
    // countItems(where:), prefix(count:), suffix(count:),
    // drop(first:), drop(last:): provided by extend Sliceable[T]

    // ========================================================================
    // ITERATION
    // ========================================================================

    /// Returns a forward iterator over the array's elements, as they are now:
    /// the iterator shares the storage, so later mutation of the array copies
    /// first and the iteration is unaffected.
    public func iter() -> ArrayIterator[T, A] {
        self.iterRange(start: 0, count: self.len())
    }

    /// Iterator over elements `[start, start + count)`, sharing the storage
    /// (the slices' `iter()`). The caller has checked the range fits.
    internal func iterRange(start start: Int64, count count: Int64) -> ArrayIterator[T, A] {
        ArrayIterator(
            storage: self.storage.clone(),
            cursor: UnsafeSliceIterator(ptr: self.ptr().offset(by: start), remaining: count),
        )
    }

    // refs(), mutatingRefs(): in array_slice.ks — they borrow the array.

    // chunks(of:), windows(of:): provided by extend Sliceable[T] — return
    // ChunksView[T] / WindowsView[T] (multi-pass; subscript via .get(i)).

    // ========================================================================
    // PARTITIONING
    // ========================================================================

    /// Reorders elements in place so that all matching elements come
    /// before all non-matching elements; returns the partition point.
    ///
    /// The returned index is the count of matching elements (and the
    /// index of the first non-matching one). This is an *unstable*
    /// partition — relative order within each side is not preserved.
    /// For a stable, allocating variant that returns two arrays, use
    /// `partitioned(by:)`.
    ///
    /// # Examples
    ///
    /// ```
    /// var arr = [1, 2, 3, 4, 5];
    /// let pivot = arr.partition(by: { (x) in x % 2 == 0 });
    /// // arr might be [2, 4, 3, 1, 5] (or another valid permutation)
    /// // pivot == 2 — first two elements satisfy the predicate
    /// ```
    public mutating func partition(by predicate: (T) -> Bool) -> Int64 {
        self.asMutableSlice().partition(by: predicate)
    }

    /// Returns two new arrays: elements matching `predicate` first, then
    /// elements that don't.
    ///
    /// Stable: relative order within each side is preserved. Allocates
    /// two new arrays — use `partition(by:)` for an in-place, unstable
    /// reordering that avoids the allocation.
    ///
    /// # Examples
    ///
    /// ```
    /// let (evens, odds) = [1, 2, 3, 4, 5].partitioned(by: { (x) in x % 2 == 0 });
    /// // evens = [2, 4]
    /// // odds  = [1, 3, 5]
    /// ```
    public func partitioned(by predicate: (T) -> Bool) -> (Array[T], Array[T]) where T: Copyable {
        var matching = Array[T]();
        var notMatching = Array[T]();
        let myLen = self.len();
        let myPtr = self.ptr();
        for i in 0..<myLen {
            let matches = predicate(unsafe myPtr.offset(by: i).value);
            if matches {
                matching.append(unsafe myPtr.offset(by: i).read())
            } else {
                notMatching.append(unsafe myPtr.offset(by: i).read())
            }
        }
        (matching, notMatching)
    }

    // ========================================================================
    // PROTOCOL CONFORMANCES
    // ========================================================================

    /// Returns an `Array[T]` sharing the same storage; the deep copy is
    /// deferred until either side mutates.
    ///
    /// O(1) — just bumps the storage `CowBox`'s refcount. The first
    /// mutation on either the original or the clone triggers
    /// `makeUnique()`, which deep-copies the buffer so the two arrays
    /// diverge.
    ///
    /// # Examples
    ///
    /// ```
    /// let a = [1, 2, 3];
    /// var b = a.clone();  // O(1), shares storage
    /// b.append(4);        // b deep-copies here; a is unchanged
    /// ```
    public func clone() -> Array[T, A] {
        Array(storage: self.storage.clone())
    }
}

// Constructors that make their own allocator: available when it has a
// default (`GlobalAllocator`, the default `A`, does), and with them the
// literal and `Defaultable` conformances.
extend Array[T, A]: ExpressibleByArrayLiteral, _ExpressibleByArrayLiteral, Defaultable where A: Defaultable {
    /// Array-literal element type.
    type Element = T

    // ========================================================================
    // CONSTRUCTORS
    // ========================================================================

    /// @name Empty
    /// Creates an empty array with no allocation.
    ///
    /// Capacity starts at zero; the first `append` allocates a small
    /// buffer (currently 4 elements). Use `init(capacity:)` if you can
    /// pre-size to avoid the early growth steps.
    ///
    /// # Examples
    ///
    /// ```
    /// var arr = Array[Int64]();
    /// arr.count;     // 0
    /// arr.capacity;  // 0
    /// ```
    public init() {
        self.storage = CowBox(ArrayStorage(
            ptr: Pointer[T].nullPointer(),
            len: 0,
            cap: 0,
            allocator: A()
        ));
    }

    /// @name With Capacity
    /// Creates an empty array with at least the requested capacity reserved.
    ///
    /// Equivalent to `Array()` followed by `reserveCapacity(...)`, but
    /// done in a single allocation. A non-positive `capacity` behaves
    /// like `init()` (no allocation). Panics if allocation fails.
    ///
    /// # Examples
    ///
    /// ```
    /// var arr = Array[Int64](capacity: 1000);
    /// arr.count;     // 0
    /// arr.capacity;  // >= 1000 — no reallocation for first 1000 appends
    /// ```
    public init(capacity capacity: Int64) {
        if capacity > 0 {
            let layout = Layout.array[T](capacity);
            var allocator = A();
            let result = allocator.allocate(layout);
            if let .Some(rawPtr) = result {
                self.storage = CowBox(ArrayStorage(
                    ptr: rawPtr.cast[T](),
                    len: 0,
                    cap: capacity,
                    allocator: allocator
                ))
            } else {
                fatalError("Array allocation failed")
            }
        } else {
            self.storage = CowBox(ArrayStorage(
                ptr: Pointer[T].nullPointer(),
                len: 0,
                cap: 0,
                allocator: A()
            ))
        }
    }

    /// @name Literal Bridge
    /// Compiler-emitted bridge initializer for `[a, b, c]` array literals.
    ///
    /// Not called by user code directly — the parser lowers literal
    /// expressions into a `(ptr, count)` pair which this constructor wraps
    /// in a `LiteralSlice` and forwards to `init(arrayLiteral:)`.
    ///
    /// # Safety
    ///
    /// The compiler guarantees `_arrayLiteralPointer` points to exactly
    /// `_arrayLiteralCount` initialized elements of `T`.
    ///
    /// # Examples
    ///
    /// ```
    /// let arr = [1, 2, 3];   // emitted by the compiler as a call to this init
    /// ```
    public init(consuming _arrayLiteralPointer _arrayLiteralPointer: lang.ptr[T], consuming _arrayLiteralCount _arrayLiteralCount: lang.i64) {
        // The compiler hands us an OWNED, initialized buffer of `count`
        // elements (it moved each element in and abandons the buffer after
        // this call). Bit-MOVE the whole buffer into fresh storage with a
        // single memcpy. Do NOT route through `init(arrayLiteral:)` /
        // `LiteralSlice` element-wise `read()`: `read()` clones for
        // non-Copyable `T`, which would duplicate every element and leak the
        // abandoned source originals (the buffer is never dropped). A bitwise
        // transfer moves ownership exactly once — no clone, no leak.
        let count = Int64(intLiteral: _arrayLiteralCount);
        if count > 0 {
            let layout = Layout.array[T](count);
            var allocator = A();
            if let .Some(rawPtr) = allocator.allocate(layout) {
                let newPtr = rawPtr.cast[T]();
                let stride = Int64(intLiteral: lang.sizeof[T]());
                 unsafe memcpy(newPtr.asRaw(), Pointer[T](raw: _arrayLiteralPointer).asRaw(), count * stride);
                self.storage = CowBox(ArrayStorage(ptr: newPtr, len: count, cap: count, allocator: allocator))
            } else {
                fatalError("Array allocation failed")
            }
        } else {
            self.storage = CowBox(ArrayStorage(
                ptr: Pointer[T].nullPointer(),
                len: 0,
                cap: 0,
                allocator: A()
            ))
        }
    }

    /// @name Array Literal
    /// Creates an array containing every element of the supplied literal
    /// slice.
    ///
    /// Allocates a buffer sized exactly to the literal's element count
    /// (so `capacity == count` after construction) and copies the
    /// elements over. An empty slice yields an empty unallocated array.
    /// Panics if allocation fails.
    ///
    /// # Examples
    ///
    /// ```
    /// // Triggered by the array-literal syntax:
    /// let arr: Array[Int64] = [10, 20, 30];
    /// ```
    public init(arrayLiteral elements: LiteralSlice[T]) {
        let elementCount = elements.count;
        if elementCount > 0 {
            let layout = Layout.array[T](elementCount);
            var allocator = A();
            let result = allocator.allocate(layout);
            if let .Some(rawPtr) = result {
                let newPtr = rawPtr.cast[T]();
                var currentLen: Int64 = 0;
                // Copy elements from literal slice
                var iter = elements.iter();
                while let .Some(item) = iter.next() {
                    unsafe newPtr.offset(by: currentLen).write(item);
                    currentLen = currentLen + 1
                }
                self.storage = CowBox(ArrayStorage(
                    ptr: newPtr,
                    len: currentLen,
                    cap: elementCount,
                    allocator: allocator
                ))
            } else {
                fatalError("Array allocation failed")
            }
        } else {
            self.storage = CowBox(ArrayStorage(
                ptr: Pointer[T].nullPointer(),
                len: 0,
                cap: 0,
                allocator: A()
            ))
        }
    }

    /// @name Repeating Value
    /// Creates an array of `count` identical copies of `value`.
    ///
    /// Allocates exactly `count` slots and writes the same value into each.
    /// `count <= 0` produces an empty array. Useful for initializing
    /// fixed-size buffers; if you instead want each slot computed, use
    /// `init(count:generator:)`.
    ///
    /// # Examples
    ///
    /// ```
    /// let zeros = Array(repeating: 0, count: 5);    // [0, 0, 0, 0, 0]
    /// let empty = Array(repeating: "x", count: 0);  // []
    /// let pad   = Array(repeating: " ", count: 3);  // [" ", " ", " "]
    /// ```
    public init(repeating value: T, count count: Int64) {
        self.init(repeating: value, count: count, allocator: A())
    }

    /// @name From Iterable
    /// Creates an array by collecting every element produced by an iterable.
    ///
    /// Drains `iterable` to completion via `append`, so the resulting
    /// capacity is whatever the growth policy lands on (not necessarily
    /// equal to `count`). For a sized source you can shave reallocations
    /// by following with `shrinkToFit()`. See also `append(from:)`
    /// to add elements to an existing array.
    ///
    /// # Examples
    ///
    /// ```
    /// let fromRange = Array(from: 1..<5);         // [1, 2, 3, 4]
    /// let fromSet   = Array(from: mySet);         // arbitrary order
    /// let collected = lines.iter().collect();     // an iterator: use collect()
    /// ```
    public init[I](from iterable: I) where I: Iterable, I: not Static, I.Item = T {
        self.init();
        var iter = iterable.iter();
        while let .Some(item) = iter.next() {
            self.append(item)
        }
    }

    /// @name From Generator
    /// Creates an array of `count` elements computed by a per-index closure.
    ///
    /// Allocates exactly `count` slots and invokes `gen(i)` once for each
    /// `i` in `0..<count`. `count <= 0` produces an empty array. Use this
    /// when each slot is a function of its index; for a constant value,
    /// prefer `init(repeating:count:)`.
    ///
    /// # Examples
    ///
    /// ```
    /// let squares = Array(of: 5, generatedBy: { (i) in i * i });  // [0, 1, 4, 9, 16]
    /// let indices = Array(of: 3, generatedBy: { (i) in i });      // [0, 1, 2]
    /// let empty   = Array(of: 0, generatedBy: { (i) in i });      // []
    /// ```
    public init(of count: Int64, generatedBy gen: (Int64) -> T) {
        if count <= 0 {
            self.init()
        } else {
            let layout = Layout.array[T](count);
            var allocator = A();
            let result = allocator.allocate(layout);
            if let .Some(rawPtr) = result {
                let newPtr = rawPtr.cast[T]();
                for i in 0..<count {
                    unsafe newPtr.offset(by: i).write(gen(i));
                }
                self.storage = CowBox(ArrayStorage(
                    ptr: newPtr,
                    len: count,
                    cap: count,
                    allocator: allocator
                ))
            } else {
                fatalError("Array allocation failed")
            }
        }
    }
}

// ============================================================================
// CONDITIONAL EXTENSIONS
// ============================================================================

/// `Equatable` and value-based search/dedup operations available when the
/// element type itself is `Equatable`.
// contains, firstIndex(of:), lastIndex(of:), starts(with:), ends(with:), split(separator:),
// isEqual: provided by extend Sliceable[T] where T: Equatable
extend Array[T, A]: Equatable where T: Equatable { }

extend Array[T, A] where T: Equatable {
    /// Removes the first element equal to `element`. Returns whether a
    /// removal occurred.
    ///
    /// Performs `firstIndex(of:)` then `remove(at:)`. To strip every
    /// occurrence in one pass, use `removeAll(element:)`.
    ///
    /// # Examples
    ///
    /// ```
    /// var arr = [1, 2, 3, 2];
    /// arr.remove(2);  // true; arr is [1, 3, 2]
    /// arr.remove(5);  // false; arr unchanged
    /// ```
    public mutating func remove(element: T) -> Bool {
        if let .Some(idx) = self.firstIndex(of: element) {
             self.remove(at: idx);
            true
        } else {
            false
        }
    }

    /// Removes every element equal to `element`.
    ///
    /// Implemented as `retain` with a negated equality predicate —
    /// O(n), single pass, stable. To remove only the first occurrence
    /// use `remove(element:)`.
    ///
    /// # Examples
    ///
    /// ```
    /// var arr = [1, 2, 3, 2, 4, 2];
    /// arr.removeAll(2);  // [1, 3, 4]
    /// ```
    public mutating func removeAll(element: T) {
        self.retain(where: { (x) in x.isEqual(to: element) == false })
    }

    /// Removes runs of consecutive equal elements, in place.
    ///
    /// Only adjacent duplicates collapse — non-adjacent equal values are
    /// kept. To deduplicate globally, `sort()` first or, for `Hashable`
    /// elements, use the `unique()` / `removeDuplicates()` extension
    /// methods. The non-mutating variant is `deduped()`.
    ///
    /// # Examples
    ///
    /// ```
    /// var arr = [1, 1, 2, 2, 2, 3, 1, 1];
    /// arr.dedup();  // [1, 2, 3, 1] — trailing 1s survive (not adjacent to first run)
    /// ```
    public mutating func dedup() {
        if self.count <= 1 {
            return
        }
        self.makeUnique();
        let s = &mutating self.storage.value;
        var writeIdx: Int64 = 1;
        for readIdx in 1..<s.len {
            // Compare in place; keepers move down, duplicates are dropped
            // (copying here leaked every element it touched).
            let current = s.ptr.offset(by: readIdx);
            let duplicate = unsafe current.value.isEqual(to: s.ptr.offset(by: writeIdx - 1).value);
            if duplicate {
                unsafe current.dropInPlace()
            } else {
                if writeIdx != readIdx {
                    unsafe s.ptr.offset(by: writeIdx).write(current.take())
                }
                writeIdx = writeIdx + 1
            }
        }
        s.len = writeIdx
    }

    /// Returns a new array with consecutive duplicates removed; original
    /// is unchanged.
    ///
    /// Non-mutating mirror of `dedup()`. Same caveat: only adjacent
    /// duplicates collapse.
    ///
    /// # Examples
    ///
    /// ```
    /// [1, 1, 2, 2, 3].deduped();        // [1, 2, 3]
    /// [1, 2, 1, 2].deduped();           // [1, 2, 1, 2] — none are adjacent
    /// ```
    public func deduped() -> Array[T, A] {
        var result = self.clone();
        result.dedup();
        result
    }
}

/// `ArrayMatchable` conformance — wires `Array[T]` into the compiler's
/// array-pattern matcher.
///
/// Enables patterns such as `[a, b]`, `[a, ..rest]`, `[a, .., z]`, and
/// `[a, ..rest, z]` against an `Array[T]` scrutinee. End users do not call
/// the methods below directly — they're invoked by lowered match code.
extend Array[T, A]: unsafe ArrayMatchable {
    /// `ArrayMatchable` element type — what the pattern bindings extract.
    type Element = T
    /// `..rest` binds a slice of the array's elements.
    type Rest = &Slice[T]

    /// Pattern-matcher hook returning the array's `count`.
    ///
    /// Used by the matcher to decide whether the scrutinee has enough
    /// elements for a fixed-arity pattern.
    public func matchLength() -> Int64 {
        self.count
    }

    /// Pattern-matcher hook reading the element at `index` (no bounds
    /// check).
    ///
    /// # Safety
    ///
    /// The matcher only calls this with indices it has already validated
    /// against `matchLength()`, so the unchecked read is safe in that
    /// context.
    public func matchGet(index: Int64) -> T {
        unsafe self(unchecked: index)
    }

    /// Pattern-matcher hook returning the half-open `[from, to)` slice.
    ///
    /// Used to bind `..rest` segments. The matcher guarantees the
    /// indices are in range.
    public func matchSlice(from: Int64, to: Int64) -> &Slice[T] {
        self(from..<to)
    }
}

// ============================================================================
// COMPARABLE EXTENSION
// ============================================================================

/// Ordering-aware operations available when `T: Comparable`.
extend Array[T, A] where T: Comparable {
    /// Sorts the array in ascending order using the natural `<` ordering.
    ///
    /// Uses introsort — O(n log n) worst-case. For descending or custom
    /// orderings pass a comparator to `sort(by:)`. Non-mutating variant:
    /// `sorted()`.
    ///
    /// # Examples
    ///
    /// ```
    /// var arr = [3, 1, 4, 1, 5];
    /// arr.sort();  // [1, 1, 3, 4, 5]
    /// ```
    public mutating func sort() {
        self.sort(by: { (a, b) in a.compare(b) == Ordering.Less })
    }

    // sorted(), min(), max(), isSorted(), binarySearch(): provided by extend Sliceable[T] where T: Comparable
}

// ============================================================================
// HASH EXTENSION
// ============================================================================

/// `Hashable` conformance — an array hashes its elements in order, so
/// two arrays hash equal exactly when they compare `==`.
extend Array[T, A]: Hashable where T: Hashable {
    /// Order-sensitive hash: feeds the element count, then each element
    /// front to back, into `hasher`.
    ///
    /// The count prefix disambiguates nested-collection layouts (e.g.
    /// `[[1], [2]]` vs `[[1, 2], []]`). Consistent with the `Equatable`
    /// conformance: equal arrays produce equal hashes.
    ///
    /// # Examples
    ///
    /// ```
    /// var h = DefaultHasher();
    /// [1, 2, 3].hash(into: h);
    /// let digest = h.finish();
    /// ```
    public func hash[H](mutating into hasher: H) where H: Hasher {
        self.count.hash(into: hasher);
        for i in 0..<self.count {
            unsafe self(unchecked: i).hash(into: hasher);
        }
    }
}

extend Array[T, A] where T: Hashable {
    /// Removes every duplicate in place, keeping the first occurrence.
    ///
    /// Implemented by replacing storage with the result of `unique()`,
    /// so it's O(n) expected. The non-mutating mirror is `unique()`.
    ///
    /// # Examples
    ///
    /// ```
    /// var arr = [1, 2, 1, 3, 2];
    /// arr.removeDuplicates();  // [1, 2, 3]
    /// ```
    public mutating func removeDuplicates() {
        // Refill in place so the array keeps its own allocator.
        let kept = self.unique();
        self.clear();
        self.append(contentsOf: kept)
    }
}

// ============================================================================
// CUSTOM SORTING EXTENSION
// ============================================================================

/// Custom-comparator and key-extracting sort overloads, available for
/// every element type (no `Comparable` requirement).
extend Array[T, A] {
    /// Sorts the array in place using a `<`-style comparator.
    ///
    /// The comparator returns `true` when its first argument should come
    /// before the second. Uses introsort — quicksort with heapsort
    /// fallback when recursion exceeds 2·log₂(n), and insertion sort for
    /// partitions ≤ 16 elements. O(n log n) worst-case. Pass a reversed
    /// comparator for descending order.
    ///
    /// # Examples
    ///
    /// ```
    /// var arr = [1, 5, 3, 2, 4];
    /// arr.sort(by: { (a, b) in a > b });  // [5, 4, 3, 2, 1] descending
    /// ```
    public mutating func sort(by comparator: (T, T) -> Bool) {
        self.asMutableSlice().sort(by: comparator)
    }

    // -- Public sort variants -----------------------------------------------

    /// Returns a new array sorted by a custom comparator. Original unchanged.
    ///
    /// # Examples
    ///
    /// ```
    /// let arr = [3, 1, 2];
    /// let desc = arr.sorted(by: { (a, b) in a > b });  // [3, 2, 1]
    /// ```
    public func sorted(by comparator: (T, T) -> Bool) -> Array[T, A] {
        var result = self.clone();
        result.sort(by: comparator);
        result
    }

    /// Sorts the array in place by an extracted `Comparable` key.
    ///
    /// # Examples
    ///
    /// ```
    /// var people = [Person("Alice", 30), Person("Bob", 25)];
    /// people.sort(byKey: { (p) in p.age });
    /// ```
    public mutating func sort[K](consuming byKey key: (T) -> K) where K: Comparable {
        let n = self.count;
        if n < 2 {
            return
        }
        // Key each element once (n calls, not two per comparison), sort the
        // (key, index) pairs, then apply that permutation with swaps. Ties
        // keep their original order.
        var order = Array[(K, Int64)](capacity: n);
        for i in 0..<n {
            order.append((key(self(i)), i))
        }
        order.sort(by: { (a, b) in
            let c = a.0.compare(b.0);
            c == Ordering.Less or (c == Ordering.Equal and a.1 < b.1)
        });
        // perm(j) is the old index of the element that belongs at j.
        var perm = Array[Int64](capacity: n);
        for j in 0..<n {
            perm.append(order(j).1)
        }
        for i in 0..<n {
            var cur = i;
            while perm(cur) != i {
                let next = perm(cur);
                self.swap(at: cur, with: next);
                perm(cur) = cur;
                cur = next
            }
            perm(cur) = cur
        }
    }

    /// Returns a new array sorted by an extracted `Comparable` key;
    /// original unchanged.
    ///
    /// # Examples
    ///
    /// ```
    /// let words = ["hi", "hello", "hey"];
    /// let byLen = words.sorted(byKey: { (w) in w.count });
    /// ```
    public func sorted[K](consuming byKey key: (T) -> K) -> Array[T, A] where K: Comparable {
        var copy = self.clone();
        copy.sort(byKey: key);
        copy
    }
}

// ============================================================================
// NESTED STRUCTURE EXTENSIONS
// ============================================================================

/// Flattening for arrays whose elements are themselves `Iterable`.
extend Array[T, A] where T: Iterable {
    /// Concatenates each element's iterator into a single
    /// `Array[T.Item]`.
    ///
    /// Drains every inner iterator in order. Empty inner sequences
    /// disappear without affecting the surrounding ones. Element type
    /// of the result is `T.Item`, the inner iterable's item type.
    ///
    /// # Examples
    ///
    /// ```
    /// let nested = [[1, 2], [3, 4], [5]];
    /// nested.flatten();  // [1, 2, 3, 4, 5]
    ///
    /// let mixed = [[1], [], [2, 3]];
    /// mixed.flatten();   // [1, 2, 3]
    /// ```
    public func flatten() -> Array[T.Item] {
        var result = Array[T.Item]();
        for i in 0..<self.count {
            var iter = unsafe self(unchecked: i).iter();
            while let .Some(item) = iter.next() {
                result.append( item)
            }
        }
        result
    }
}

/// String-joining for arrays whose elements are `Formattable`.
extend Array[T, A] where T: Formattable {
    /// Concatenates each element's string representation, separated by
    /// `separator`.
    ///
    /// Each element is rendered with its `format()` method using default
    /// `FormatOptions`. The default `separator` is empty (raw
    /// concatenation). Empty arrays produce `""`. For the bracketed
    /// debug form (`"[1, 2, 3]"`), use `format()` directly.
    ///
    /// # Examples
    ///
    /// ```
    /// [1, 2, 3].joined(", ");  // "1, 2, 3"
    /// [1, 2, 3].joined();       // "123"
    /// ["a", "b"].joined("-");   // "a-b"
    /// [].joined(", ");          // ""
    /// ```
    public func joined(separator: String = "") -> String {
        if self.count == 0 {
            return ""
        }
        var b = StringBuilder();
        unsafe self(unchecked: 0).format(into: b);
        for i in 1..<self.count {
            b.append(separator);
            unsafe self(unchecked: i).format(into: b)
        }
        b.build()
    }
}

/// Renders as `"[e1, e2, ...]"`, so arrays work in string interpolation and
/// `print`. The `format` witness comes from `extend Sliceable[T] where
/// T: Formattable`.
extend Array[T, A]: Formattable where T: Formattable {}

// ============================================================================
// DIRECT ITERABLE CONFORMANCE
// ============================================================================

// TODO: DirectIterable protocol not yet implemented
// /// DirectIterable conformance allows using iterator methods directly on arrays.
// extend Array[T]: DirectIterable[T] {
//     public static func collect[I](from iter: I) -> Array[T] where I: Iterator, I.Item = T {
//         var result = Array[T]();
//         var iterator = iter;
//         while let .Some(item) = iterator.next() {
//             result.append( item)
//         }
//         result
//     }
// }

// ============================================================================
// TYPE OPERATOR
// ============================================================================

/// Compiler-recognized type alias that lets `[T]` desugar to `Array[T]`.
///
/// Allows annotations like `let xs: [Int64] = [1, 2, 3]` instead of
/// requiring the user to spell out `Array[Int64]`. Not intended for
/// direct use — the parser inserts it automatically when it sees the
/// `[T]` shorthand in a type position.
///
/// # Examples
///
/// ```
/// let xs: [Int64] = [1, 2, 3];   // same as: Array[Int64]
/// func sum(of values: [Float]) -> Float { ... }
/// ```
@builtin(.ArrayTypeOperator)
public type ArrayTypeOperator[T] = Array[T, GlobalAllocator];

/// Whether the `size` bytes at `p` are all zero.
func bytesAreZero(p: RawPointer, size: Int64) -> Bool {
    let bytes = p.cast[UInt8]();
    for i in 0..<size {
        if unsafe bytes.offset(by: i).read() != 0 { return false }
    }
    true
}
