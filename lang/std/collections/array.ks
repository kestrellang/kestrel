// Array[T] - dynamic growable array with COW (Copy-on-Write) semantics

module std.collections

import std.core.(Bool, Equatable, Comparable, Cloneable, ArrayMatchable, Defaultable, fatalError)
import std.core.(ExpressibleByArrayLiteral, _ExpressibleByArrayLiteral)
import std.core.(Range, ClosedRange, Hash)
import std.text.(Formattable, FormatOptions)
import std.numeric.(Int64)
import std.numeric.(RandomNumberGenerator, Lcg64)
import std.result.(Optional)
import std.memory.(Layout, Pointer, Slice, RawPointer, SystemAllocator, LiteralSlice, RcBox)
import std.iter.(Iterator, Iterable)
import std.text.(String)

// ============================================================================
// ARRAY ITERATOR
// ============================================================================

/// Single-pass forward iterator over the elements of an `Array[T]`.
///
/// Produced by `Array.iter()`, walks the underlying storage one element at a
/// time and yields owned copies of each element. The iterator holds a raw
/// pointer into the array's buffer, so any mutation of the source array
/// (which may reallocate) invalidates iteration. Use `chunks(of:)` or
/// `windows(of:)` if you need grouped views instead.
///
/// # Examples
///
/// ```
/// let arr = [1, 2, 3];
/// var it = arr.iter();
/// it.next();  // Some(1)
/// it.next();  // Some(2)
/// it.next();  // Some(3)
/// it.next();  // None
/// ```
///
/// # Representation
///
/// A `(ptr, remaining)` pair: a `Pointer[T]` advanced on each call and an
/// `Int64` count of remaining elements.
///
/// # Memory Model
///
/// Value type. The pointer aliases array storage; do not retain an iterator
/// across mutations of the source array.
public struct ArrayIterator[T]: Iterator {
    /// The element type yielded by `next()` — always `T`.
    type Item = T

    /// Pointer to the next element to return; advances on each `next()` call.
    private var ptr: Pointer[T]
    /// Number of elements still to yield.
    private var remaining: Int64

    /// @name From Pointer
    /// Constructs an iterator from a raw pointer and a remaining-count.
    ///
    /// Normally you should not call this directly — use `Array.iter()` instead.
    /// The pointer must be valid for `remaining` reads of `T`.
    ///
    /// # Safety
    ///
    /// The caller must guarantee `ptr` points to at least `remaining`
    /// initialized elements of `T` and remains valid for the iterator's
    /// lifetime.
    ///
    /// # Examples
    ///
    /// ```
    /// let it = ArrayIterator(ptr: arr.asPointer(), remaining: arr.count);
    /// ```
    public init(ptr ptr: Pointer[T], remaining remaining: Int64) {
        self.ptr = ptr;
        self.remaining = remaining;
    }

    /// Advances the iterator and returns the next element, or `None` when the
    /// iterator is exhausted.
    ///
    /// Each call reads one element, advances the internal pointer by one,
    /// and decrements the remaining count. Once `None` is returned the
    /// iterator stays exhausted.
    ///
    /// # Examples
    ///
    /// ```
    /// var it = [10, 20].iter();
    /// it.next();  // Some(10)
    /// it.next();  // Some(20)
    /// it.next();  // None
    /// ```
    public mutating func next() -> T? {
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
// CHUNKS ITERATOR
// ============================================================================

/// Iterator over non-overlapping `Slice[T]` chunks of an `Array[T]`.
///
/// Produced by `Array.chunks(of:)`, walks the source buffer in fixed-size
/// strides and yields each chunk as a borrowed `Slice[T]`. The last chunk
/// may be shorter than `chunkSize` when the array length is not evenly
/// divisible. For overlapping windows of a fixed size instead, use
/// `WindowsIterator` / `Array.windows(of:)`.
///
/// # Examples
///
/// ```
/// let arr = [1, 2, 3, 4, 5];
/// for chunk in arr.chunks(of: 2) {
///     // yields: Slice[1, 2], Slice[3, 4], Slice[5]
/// }
/// ```
///
/// # Representation
///
/// A `(ptr, remaining, chunkSize)` triple: a pointer advanced by one chunk
/// per `next()` call, plus the count of unread elements and the requested
/// stride.
///
/// # Memory Model
///
/// Value type. Yielded slices alias the source array's buffer; do not
/// retain them across mutations of the array.
public struct ChunksIterator[T]: Iterator {
    /// The element type yielded by `next()` — a borrowed `Slice[T]` over
    /// one chunk.
    type Item = Slice[T]

    /// Pointer to the start of the next chunk to yield.
    private var ptr: Pointer[T]
    /// Number of source elements still unread.
    private var remaining: Int64
    /// The requested fixed stride; final chunk may be shorter.
    private var chunkSize: Int64

    /// @name From Pointer
    /// Constructs a chunks iterator from a pointer, total element count, and
    /// chunk stride.
    ///
    /// Prefer `Array.chunks(of:)` over calling this directly.
    ///
    /// # Safety
    ///
    /// `ptr` must point to at least `remaining` initialized elements of
    /// `T`, and `chunkSize` should be positive.
    ///
    /// # Examples
    ///
    /// ```
    /// let it = ChunksIterator(ptr: arr.asPointer(), remaining: arr.count, chunkSize: 2);
    /// ```
    public init(ptr ptr: Pointer[T], remaining remaining: Int64, chunkSize chunkSize: Int64) {
        self.ptr = ptr;
        self.remaining = remaining;
        self.chunkSize = chunkSize;
    }

    /// Returns the next chunk, or `None` when the source is exhausted.
    ///
    /// The returned `Slice[T]` has length `chunkSize`, except for the final
    /// chunk which may be shorter if the total count was not evenly
    /// divisible.
    ///
    /// # Examples
    ///
    /// ```
    /// var it = [1, 2, 3, 4, 5].chunks(of: 2);
    /// it.next();  // Some(Slice[1, 2])
    /// it.next();  // Some(Slice[3, 4])
    /// it.next();  // Some(Slice[5])     // shorter trailing chunk
    /// it.next();  // None
    /// ```
    public mutating func next() -> Slice[T]? {
        if self.remaining <= 0 {
            return .None
        }

        // Determine this chunk's actual size (may be smaller for last chunk)
        let thisChunkSize: Int64 = if self.remaining < self.chunkSize {
            self.remaining
        } else {
            self.chunkSize
        };

        let slice = Slice(pointer: self.ptr, count: thisChunkSize);
        self.ptr = self.ptr.offset(by: thisChunkSize);
        self.remaining = self.remaining - thisChunkSize;
        .Some(slice)
    }
}

// ============================================================================
// WINDOWS ITERATOR
// ============================================================================

/// Iterator over overlapping fixed-size sliding windows of an `Array[T]`.
///
/// Produced by `Array.windows(of:)`. Every yielded window has exactly
/// `windowSize` elements; the pointer advances by one element per step, so
/// adjacent windows overlap by `windowSize - 1` elements. If the array is
/// shorter than the window size, no windows are yielded. For
/// non-overlapping fixed-size groups, use `ChunksIterator` instead.
///
/// # Examples
///
/// ```
/// let arr = [1, 2, 3, 4];
/// for window in arr.windows(of: 2) {
///     // yields: Slice[1, 2], Slice[2, 3], Slice[3, 4]
/// }
/// ```
///
/// # Representation
///
/// A `(ptr, remaining, windowSize)` triple. `remaining` is precomputed at
/// construction as `max(totalCount - windowSize + 1, 0)`.
///
/// # Memory Model
///
/// Value type. Yielded slices alias the source array's buffer; do not
/// retain them across mutations of the array.
public struct WindowsIterator[T]: Iterator {
    /// The element type yielded by `next()` — a borrowed `Slice[T]` over
    /// one window.
    type Item = Slice[T]

    /// Pointer to the start of the next window; advances by one element
    /// per call.
    private var ptr: Pointer[T]
    /// Number of windows still to yield.
    private var remaining: Int64
    /// The fixed window length; every yielded slice has this size.
    private var windowSize: Int64

    /// @name From Pointer
    /// Constructs a windows iterator from a pointer, total element count,
    /// and window size.
    ///
    /// Prefer `Array.windows(of:)` over calling this directly. The window
    /// count is derived as `max(totalCount - windowSize + 1, 0)`, so a
    /// `windowSize` larger than `totalCount` yields nothing.
    ///
    /// # Safety
    ///
    /// `ptr` must point to at least `totalCount` initialized elements of
    /// `T` and remain valid for the iterator's lifetime.
    ///
    /// # Examples
    ///
    /// ```
    /// let it = WindowsIterator(ptr: arr.asPointer(), totalCount: arr.count, windowSize: 2);
    /// ```
    public init(ptr ptr: Pointer[T], totalCount totalCount: Int64, windowSize windowSize: Int64) {
        self.ptr = ptr;
        self.windowSize = windowSize;
        // Number of windows = totalCount - windowSize + 1 (if positive)
        let windowCount = totalCount - windowSize + 1;
        self.remaining = if windowCount > 0 {
            windowCount
        } else {
            0
        };
    }

    /// Returns the next window, or `None` when no more full windows fit.
    ///
    /// Each call slides the pointer forward by one element, so consecutive
    /// windows share `windowSize - 1` elements.
    ///
    /// # Examples
    ///
    /// ```
    /// var it = [1, 2, 3].windows(of: 2);
    /// it.next();  // Some(Slice[1, 2])
    /// it.next();  // Some(Slice[2, 3])
    /// it.next();  // None
    /// ```
    public mutating func next() -> Slice[T]? {
        if self.remaining <= 0 {
            return .None
        }

        let slice = Slice(pointer: self.ptr, count: self.windowSize);
        self.ptr = self.ptr.offset(by: 1);
        self.remaining = self.remaining - 1;
        .Some(slice)
    }
}

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
struct ArrayStorage[T]: Cloneable {
    /// Heap pointer to the element buffer; null when `cap == 0`.
    var ptr: Pointer[T]
    /// Number of initialized elements stored in the buffer.
    var len: Int64
    /// Total slots allocated; always `>= len`.
    var cap: Int64

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
    /// let s = ArrayStorage(ptr: rawPtr.cast[T](), len: 0, cap: 16);
    /// ```
    init(ptr ptr: Pointer[T], len len: Int64, cap cap: Int64) {
        self.ptr = ptr;
        self.len = len;
        self.cap = cap;
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
    func clone() -> ArrayStorage[T] {
        if self.len == 0 {
            return ArrayStorage(
                ptr: Pointer[T].nullPointer(),
                len: 0,
                cap: 0
            )
        }
        let layout = Layout.array[T](self.len);
        var allocator = SystemAllocator();
        let result = allocator.allocate(layout);
        if let .Some(rawPtr) = result {
            let newPtr = rawPtr.cast[T]();
            // Copy elements
            for i in 0..<self.len {
                newPtr.offset(by: i).write(self.ptr.offset(by: i).read());
            }
            ArrayStorage(ptr: newPtr, len: self.len, cap: self.len)
        } else {
            fatalError("ArrayStorage clone allocation failed")
        }
    }

    /// Frees the underlying buffer.
    ///
    /// Runs when the last `RcBox` reference to this storage drops. Skips
    /// the deallocation entirely when `cap == 0` (no buffer was ever
    /// allocated). Element destructors are not invoked individually here —
    /// `T` is treated as trivially droppable at the storage level.
    deinit {
        if self.cap > 0 {
            let layout = Layout.array[T](self.cap);
            var allocator = SystemAllocator();
            allocator.deallocate(self.ptr.asRaw(), layout)
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
/// existing buffer use `Slice[T]`; for fixed-size or set-like collections
/// see `Slice[T]`, `Set`, or `Dictionary`.
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
/// for every common policy: `arr(checked: i)` returns `T?`,
/// `arr(unchecked: i)` skips the bounds check (UB on OOB),
/// `arr(wrapped: i)` wraps with modulo (and supports negative indices),
/// and `arr(clamped: i)` clamps to `[0, count-1]`. Range arguments use the
/// same labels — `arr(0..<3)`, `arr(checked: r)`, `arr(unchecked: r)`,
/// `arr(clamped: r)` — dispatched through the `ArrayIndex[T]` and
/// `ArrayClampable[T]` protocols. `Int64` and range types share each
/// label; the result type varies (`T?` vs `Slice[T]` for `clamped:`).
///
/// # Capacity & Reallocation
///
/// `count` is the number of elements; `capacity` is how many can fit
/// without reallocating. When `append` would exceed capacity the buffer
/// doubles (starting from 4 if previously zero). Use
/// `reserveCapacity(minimumCapacity:)` to pre-allocate, and
/// `shrinkToFit()` to release excess.
///
/// # Representation
///
/// Holds a single `RcBox[ArrayStorage[T]]` field. The storage is a
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
// ============================================================================
// ARRAY INDEX PROTOCOL
// ============================================================================

/// Stdlib-internal index types for `Array[T]`'s default subscripts.
///
/// Conforming types describe how a value of that type reads from and
/// writes to an `Array[T]`. Used by `Array`'s generic `(i)`,
/// `(checked: i)`, and `(unchecked: i)` subscripts so a single declaration
/// covers single elements (`Int64`), half-open slices (`Range[Int64]`),
/// and closed slices (`ClosedRange[Int64]`). `Yield` is what the access
/// produces — `T` for single-element indexes, `Slice[T]` for range
/// indexes.
///
/// Sealed: this protocol is `internal` so user code can't add new index
/// types. Adding a new index shape (e.g. `..N`, `N..`) is a stdlib
/// change.
internal protocol ArrayIndex[T] {
    /// Element-or-slice type the access produces. Named `ArrayYield` rather
    /// than the more obvious `Output` because `Output` is the standard
    /// associated-type name across `Addable`/`Subtractable`/etc., and
    /// `Int64`'s conformance to those already binds `Output = Int64`.
    /// Inference's associated-type resolution looks up associated names
    /// across all conformances on the concrete type, returning the first
    /// match — so a shared name would shadow.
    type ArrayYield

    /// Read with bounds check — panics on out-of-bounds.
    func readArray(from array: Array[T]) -> ArrayYield

    /// Read with bounds check — returns `None` on out-of-bounds.
    func readArrayChecked(from array: Array[T]) -> ArrayYield?

    /// Read with no bounds check — UB on out-of-bounds.
    func readArrayUnchecked(from array: Array[T]) -> ArrayYield

    /// Write with bounds check — panics on out-of-bounds. For range
    /// indexes, also panics if the slice's length doesn't match the
    /// range's length.
    func writeArray(mutating to array: Array[T], with value: ArrayYield)

    /// Write with no bounds check — UB on out-of-bounds. For range
    /// indexes, also panics if the slice's length doesn't match the
    /// range's length.
    func writeArrayUnchecked(mutating to array: Array[T], with value: ArrayYield)
}

/// Stdlib-internal index types for `Array[T]`'s `(clamped: i)` subscript.
///
/// Clamping indexes saturate to the array's valid range rather than
/// panicking on out-of-bounds. `Yield` differs across conformances:
/// `Int64.Yield = T?` (still `None` on an empty array), `Range[Int64].Yield
/// = Slice[T]` (always a valid slice, possibly empty).
internal protocol ArrayClampable[T] {
    type ArrayClampedYield

    /// Read with bounds clamped to `[0, count)`.
    func readArrayClamped(from array: Array[T]) -> ArrayClampedYield

    /// Write with bounds clamped to `[0, count)`. No-op on an empty
    /// array. For range indexes, panics on length mismatch after
    /// clamping.
    func writeArrayClamped(mutating to array: Array[T], with value: ArrayClampedYield)
}

/// Stdlib-internal index types for `Array[T]`'s `(wrapped: i)` subscript.
///
/// Wrapping indexes use modulo arithmetic so negative indices count from
/// the end and large positive indices wrap to the start. The only
/// failure mode is an empty array — `Yield` is `T?` to surface that case.
internal protocol ArrayWrappable[T] {
    type ArrayWrappedYield

    /// Read with index wrapped via `index % count`. Returns `None` on
    /// an empty array.
    func readArrayWrapped(from array: Array[T]) -> ArrayWrappedYield

    /// Write with index wrapped via `index % count`. No-op on an empty
    /// array.
    func writeArrayWrapped(mutating to array: Array[T], with value: ArrayWrappedYield)
}

@builtin(.ArrayStruct)
public struct Array[T]: Iterable, ExpressibleByArrayLiteral, _ExpressibleByArrayLiteral, Cloneable, Defaultable {
    /// `Iterable` element type — the element produced by `iter().next()`.
    type Item = T
    /// `Iterable` iterator type — the concrete iterator returned by `iter()`.
    type TargetIterator = ArrayIterator[T]
    /// Pattern-matching element type — used by `ArrayMatchable` for
    /// `[a, b, ..rest]` patterns.
    type Element = T

    /// Refcounted storage cell holding `(ptr, len, cap)`. Sharing this
    /// between `Array` copies is what enables COW.
    fileprivate var storage: RcBox[ArrayStorage[T]]

    /// Returns the raw element pointer. Internal helper for storage access.
    fileprivate func ptr() -> Pointer[T] { self.storage.getValue().ptr }
    /// Returns the element count from the storage. Internal helper.
    fileprivate func len() -> Int64 { self.storage.getValue().len }
    /// Returns the buffer capacity from the storage. Internal helper.
    fileprivate func cap() -> Int64 { self.storage.getValue().cap }

    /// Ensures the storage is uniquely owned, deep-copying it if shared.
    ///
    /// This is the COW write barrier: every mutating method calls it
    /// before touching the buffer, so writes never leak into other
    /// `Array` copies that share the same `RcBox`. A no-op when this is
    /// the only reference.
    fileprivate mutating func makeUnique() {
        if self.storage.isUnique() == false {
            self.storage = RcBox(self.storage.getValue().clone())
        }
    }

    /// @name From Storage
    /// Wraps an existing storage box in a new `Array`. Used internally by
    /// `clone()` and other helpers that already have an `RcBox` in hand.
    private init(storage storage: RcBox[ArrayStorage[T]]) {
        self.storage = storage;
    }

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
        self.storage = RcBox(ArrayStorage(
            ptr: Pointer[T].nullPointer(),
            len: 0,
            cap: 0
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
            var allocator = SystemAllocator();
            let result = allocator.allocate(layout);
            if let .Some(rawPtr) = result {
                self.storage = RcBox(ArrayStorage(
                    ptr: rawPtr.cast[T](),
                    len: 0,
                    cap: capacity
                ))
            } else {
                fatalError("Array allocation failed")
            }
        } else {
            self.storage = RcBox(ArrayStorage(
                ptr: Pointer[T].nullPointer(),
                len: 0,
                cap: 0
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
    public init(_arrayLiteralPointer _arrayLiteralPointer: lang.ptr[T], _arrayLiteralCount _arrayLiteralCount: lang.i64) {
        self.init(arrayLiteral: LiteralSlice(pointer: _arrayLiteralPointer, count: _arrayLiteralCount))
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
            var allocator = SystemAllocator();
            let result = allocator.allocate(layout);
            if let .Some(rawPtr) = result {
                let newPtr = rawPtr.cast[T]();
                var currentLen: Int64 = 0;
                // Copy elements from literal slice
                var iter = elements.iter();
                while let .Some(item) = iter.next() {
                    newPtr.offset(by: currentLen).write(item);
                    currentLen = currentLen + 1
                }
                self.storage = RcBox(ArrayStorage(
                    ptr: newPtr,
                    len: currentLen,
                    cap: elementCount
                ))
            } else {
                fatalError("Array allocation failed")
            }
        } else {
            self.storage = RcBox(ArrayStorage(
                ptr: Pointer[T].nullPointer(),
                len: 0,
                cap: 0
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
        if count <= 0 {
            self.init()
        } else {
            let layout = Layout.array[T](count);
            var allocator = SystemAllocator();
            let result = allocator.allocate(layout);
            if let .Some(rawPtr) = result {
                let newPtr = rawPtr.cast[T]();
                // Write first element directly
                newPtr.write(value);
                // Copy for remaining elements
                for i in 1..<count {
                    newPtr.offset(by: i).write(value);
                }
                self.storage = RcBox(ArrayStorage(
                    ptr: newPtr,
                    len: count,
                    cap: count
                ))
            } else {
                fatalError("Array allocation failed")
            }
        }
    }

    /// @name From Iterable
    /// Creates an array by collecting every element produced by an iterable.
    ///
    /// Drains `iterable` to completion via `append`, so the resulting
    /// capacity is whatever the growth policy lands on (not necessarily
    /// equal to `count`). For a sized source you can shave reallocations
    /// by following with `shrinkToFit()`. See also `appendFrom(iterable:)`
    /// to add elements to an existing array.
    ///
    /// # Examples
    ///
    /// ```
    /// let fromRange = Array(from: 1..<5);         // [1, 2, 3, 4]
    /// let fromSet   = Array(from: mySet);         // arbitrary order
    /// let collected = Array(from: lines.iter());  // exhausts the iterator
    /// ```
    public init[I](from iterable: I) where I: Iterable, I.Item = T {
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
            var allocator = SystemAllocator();
            let result = allocator.allocate(layout);
            if let .Some(rawPtr) = result {
                let newPtr = rawPtr.cast[T]();
                for i in 0..<count {
                    newPtr.offset(by: i).write(gen(i));
                }
                self.storage = RcBox(ArrayStorage(
                    ptr: newPtr,
                    len: count,
                    cap: count
                ))
            } else {
                fatalError("Array allocation failed")
            }
        }
    }

    // ========================================================================
    // PROPERTIES
    // ========================================================================

    /// The number of elements currently in the array. Read-only; O(1).
    ///
    /// Reflects only initialized elements, not capacity. To check
    /// emptiness without comparing to zero, prefer `isEmpty`.
    ///
    /// # Examples
    ///
    /// ```
    /// [1, 2, 3].count;  // 3
    /// [].count;         // 0
    /// ```
    public var count: Int64 { get { self.len() } }

    /// The number of elements the buffer can hold without reallocating.
    ///
    /// Always `>= count`. When `append` would push `count` past
    /// `capacity` the buffer doubles (or jumps from 0 to 4). Use
    /// `reserveCapacity(...)` to pre-grow and `shrinkToFit()` to release
    /// excess. The exact value after `init(capacity:)` may exceed the
    /// requested amount because allocation rounds up.
    ///
    /// # Examples
    ///
    /// ```
    /// let arr = Array[Int64](capacity: 10);
    /// arr.capacity;  // >= 10
    /// arr.count;     // 0
    /// ```
    public var capacity: Int64 { self.cap() }

    /// `true` when the array has no elements; equivalent to `count == 0`.
    ///
    /// Reads more naturally than the comparison and is preferred in
    /// guards and predicates.
    ///
    /// # Examples
    ///
    /// ```
    /// [].isEmpty;                // true
    /// [1].isEmpty;               // false
    /// Array[Int64]().isEmpty;    // true
    /// ```
    public var isEmpty: Bool { self.len() == 0 }

    /// The valid index range `0..<count` as a `Range[Int64]`.
    ///
    /// Convenient for index-based iteration or for passing to
    /// `arr(range:)`. The range is empty for an empty array.
    ///
    /// # Examples
    ///
    /// ```
    /// let arr = [10, 20, 30];
    /// arr.indices;  // 0..<3
    ///
    /// for i in arr.indices {
    ///         print(arr(i));
    /// }
    /// ```
    public var indices: Range[Int64] {
        Range(0, self.len())
    }

    // ========================================================================
    // ACCESSORS
    // ========================================================================

    /// Returns a raw pointer to the contiguous element buffer.
    ///
    /// Intended for FFI or low-level memory work. Any operation that may
    /// reallocate (`append`, `insert`, `reserveCapacity`, `shrinkToFit`,
    /// or any mutation through a shared `Array` that triggers COW)
    /// invalidates the pointer. For a higher-level borrowed view, use
    /// `asSlice()`.
    ///
    /// # Safety
    ///
    /// The pointer outlives the array no further than the next mutation.
    /// Reading past `count` is undefined behavior; writing through the
    /// pointer skips COW and may silently mutate other `Array` copies
    /// that share the same storage.
    ///
    /// # Examples
    ///
    /// ```
    /// let p = arr.asPointer();
    /// c_sum(p, arr.count);   // pass to a C function
    /// ```
    public func asPointer() -> Pointer[T] { self.ptr() }

    /// Returns a `Slice[T]` over the entire array.
    ///
    /// The slice borrows the array's buffer; reallocation invalidates
    /// it. For a sub-range, use a range subscript such as `arr(0..<n)`.
    ///
    /// # Examples
    ///
    /// ```
    /// let arr = [1, 2, 3];
    /// let slice = arr.asSlice();  // Slice over [1, 2, 3]
    /// ```
    public func asSlice() -> Slice[T] {
        Slice(pointer: self.ptr(), count: self.len())
    }

    /// `true` if `index` is in `[0, count)`.
    ///
    /// Equivalent to `index >= 0 and index < count`. Pair with
    /// `arr(unchecked: i)` to skip a redundant bounds check after you've
    /// already validated the index.
    ///
    /// # Examples
    ///
    /// ```
    /// let arr = [1, 2, 3];
    /// arr.isValidIndex(0);   // true
    /// arr.isValidIndex(2);   // true
    /// arr.isValidIndex(3);   // false
    /// arr.isValidIndex(-1);  // false
    /// ```
    public func isValidIndex(index: Int64) -> Bool {
        index >= 0 and index < self.len()
    }

    // ========================================================================
    // ELEMENT SUBSCRIPTS
    // ========================================================================

    /// @name Indexed
    /// Reads or writes at `index`, panicking on out-of-bounds.
    ///
    /// The default subscript: trades safety for ergonomics. Dispatches via
    /// the `ArrayIndex[T]` protocol — `Int64` reads/writes a single
    /// element, `Range[Int64]` and `ClosedRange[Int64]` read or replace a
    /// `Slice[T]`. Range writes require the source slice's length to
    /// match the range's length and panic otherwise. Use
    /// `arr(checked: i)` for an `Optional` instead of a panic, or
    /// `arr(unchecked: i)` to skip the bounds check entirely. Setters
    /// trigger COW; if storage is shared the buffer is cloned before the
    /// write lands.
    ///
    /// # Errors
    ///
    /// Panics with `"Array index out of bounds"` (Int64) or
    /// `"Array range out of bounds"` (Range / ClosedRange) if the access
    /// falls outside `[0, count)`. Range writes also panic if the source
    /// slice's length doesn't match the range's length.
    ///
    /// # Examples
    ///
    /// ```
    /// var arr = [10, 20, 30, 40, 50];
    /// arr(0);                        // 10
    /// arr(1) = 25;                   // [10, 25, 30, 40, 50]
    /// arr(1..<4);                    // Slice[25, 30, 40]
    /// arr(1..<4) = otherSlice3;      // splice in three elements
    /// arr(5);                        // PANIC: index out of bounds
    /// ```
    public subscript[I](index: I) -> I.ArrayYield where I: ArrayIndex[T] {
        get { index.readArray(from: self) }
        set { index.writeArray(to: self, with: newValue) }
    }

    /// @name Checked Index
    /// Reads at `index`, returning `None` on out-of-bounds.
    ///
    /// The non-panicking counterpart to `arr(i)`. Read-only; for fallible
    /// writes pattern-match the result and assign through the default
    /// subscript. Single-element indexes return `T?`; range indexes
    /// return `Slice[T]?`. Prefer this when `index` may come from
    /// untrusted input.
    ///
    /// # Examples
    ///
    /// ```
    /// let arr = [10, 20, 30];
    /// arr(checked: 0);       // Some(10)
    /// arr(checked: 5);       // None
    /// arr(checked: 0..<2);   // Some(Slice[10, 20])
    /// arr(checked: 0..<10);  // None
    ///
    /// if let .Some(v) = arr(checked: i) {
    ///     // ...
    /// }
    /// ```
    public subscript[I](checked index: I) -> I.ArrayYield? where I: ArrayIndex[T] {
        get { index.readArrayChecked(from: self) }
    }

    /// @name Unchecked Index
    /// Reads or writes at `index` without a bounds check.
    ///
    /// The fastest accessor; intended for hot loops where the index has
    /// already been validated (e.g. inside `0..<count`). Setters trigger
    /// COW. Range writes still panic on length mismatch — that's a
    /// definitional check, not a bounds check.
    ///
    /// # Safety
    ///
    /// Undefined behavior if the access is out of range. Always validate
    /// before calling.
    ///
    /// # Examples
    ///
    /// ```
    /// let arr = [10, 20, 30];
    /// for i in arr.indices {
    ///     let v = arr(unchecked: i);   // safe — i is in range
    /// }
    /// let s = arr(unchecked: 0..<2);   // Slice[10, 20]
    /// ```
    public subscript[I](unchecked index: I) -> I.ArrayYield where I: ArrayIndex[T] {
        get { index.readArrayUnchecked(from: self) }
        set { index.writeArrayUnchecked(to: self, with: newValue) }
    }

    /// @name Clamping
    /// Reads or writes at `index` with bounds saturated to `[0, count)`.
    ///
    /// Never panics on out-of-bounds. For `Int64`, indices below `0`
    /// clamp up and indices `>= count` clamp down; an empty array yields
    /// `None`. For `Range[Int64]`, both endpoints clamp into `[0, count]`
    /// and the result is a (possibly empty) `Slice[T]`. Compare
    /// `arr(wrapped: i)`, which wraps instead of saturating.
    ///
    /// # Examples
    ///
    /// ```
    /// let arr = [10, 20, 30];
    /// arr(clamped: -5);          // Some(10) — clamped to first
    /// arr(clamped: 100);         // Some(30) — clamped to last
    /// arr(clamped: 1);           // Some(20) — in range
    /// [](clamped: 0);            // None     — empty array
    ///
    /// arr(clamped: -5..<100);    // Slice over the whole array
    /// arr(clamped: -5..<1);      // Slice[10]
    /// arr(clamped: 10..<20);     // empty Slice (both clamp to 3)
    /// ```
    public subscript[I](clamped index: I) -> I.ArrayClampedYield where I: ArrayClampable[T] {
        get { index.readArrayClamped(from: self) }
        set { index.writeArrayClamped(to: self, with: newValue) }
    }

    /// @name Wrapping
    /// Reads or writes at `index` using modulo-wrapping indexing.
    ///
    /// Negative indices count from the end (`-1` is the last element);
    /// positive indices `>= count` wrap to the start. The only failure
    /// mode is an empty array, which yields `None` (and no-ops on the
    /// setter). Compare `arr(clamped: i)`, which saturates instead of
    /// wrapping.
    ///
    /// # Examples
    ///
    /// ```
    /// let arr = [10, 20, 30];
    /// arr(wrapped: -1);  // Some(30) — last element
    /// arr(wrapped: -2);  // Some(20) — second to last
    /// arr(wrapped:  3);  // Some(10) — wraps to index 0
    /// arr(wrapped:  4);  // Some(20) — wraps to index 1
    /// [](wrapped: 0);    // None     — empty array
    /// ```
    public subscript[I](wrapped index: I) -> I.ArrayWrappedYield where I: ArrayWrappable[T] {
        get { index.readArrayWrapped(from: self) }
        set { index.writeArrayWrapped(to: self, with: newValue) }
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
    private mutating func grow(minCapacity: Int64) {
        let myCap = self.cap();
        if myCap >= minCapacity {
            return
        }

        self.makeUnique();

        // Calculate new capacity
        var newCap: Int64 = myCap;
        if newCap == 0 {
            newCap = 4
        }
        while newCap < minCapacity {
            newCap = newCap * 2
        }

        // Allocate new buffer
        let newLayout = Layout.array[T](newCap);
        var allocator = SystemAllocator();
        let result = allocator.allocate(newLayout);
        if let .Some(rawPtr) = result {
            let newPtr = rawPtr.cast[T]();
            let oldStorage = self.storage.getValue();
            // Copy existing elements
            for i in 0..<oldStorage.len {
                newPtr.offset(by: i).write(oldStorage.ptr.offset(by: i).read());
            }
            // Free old buffer
            if oldStorage.cap > 0 {
                let oldLayout = Layout.array[T](oldStorage.cap);
                allocator.deallocate(oldStorage.ptr.asRaw(), oldLayout)
            }
            self.storage.setValue(ArrayStorage(ptr: newPtr, len: oldStorage.len, cap: newCap))
        } else {
            fatalError("Array grow failed")
        }
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
    /// `appendFrom(iterable:)`.
    ///
    /// # Examples
    ///
    /// ```
    /// var arr = [1, 2];
    /// arr.append(3);  // [1, 2, 3]
    /// ```
    public mutating func append(element: T) {
        let myLen = self.len();
        self.makeUnique();
        self.grow(myLen + 1);
        var s = self.storage.getValue();
        s.ptr.offset(by: s.len).write(element);
        s.len = s.len + 1;
        self.storage.setValue(s)
    }

    /// Appends every element of `other` to the end of this array.
    ///
    /// Reserves the exact required capacity in one growth step then
    /// copies the elements over, so it's faster than calling `append`
    /// in a loop. Sharing semantics: `other` is read-only here, but if
    /// `self` shares storage with anything else, COW fires once at the
    /// start. See also `appendFrom(iterable:)` for arbitrary iterable
    /// sources.
    ///
    /// # Examples
    ///
    /// ```
    /// var arr = [1, 2];
    /// arr.append(contentsOf: [3, 4]);  // [1, 2, 3, 4]
    /// arr.append(contentsOf: []);      // [1, 2, 3, 4]  — no-op
    /// ```
    public mutating func append(contentsOf other: Array[T]) {
        let otherLen = other.count;
        if otherLen == 0 {
            return
        }
        let myLen = self.len();
        self.makeUnique();
        self.grow(myLen + otherLen);
        var s = self.storage.getValue();
        let otherPtr = other.asPointer();
        for i in 0..<otherLen {
            s.ptr.offset(by: s.len).write(otherPtr.offset(by: i).read());
            s.len = s.len + 1
        }
        self.storage.setValue(s)
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
    /// arr.appendFrom(3..<6);  // [1, 2, 3, 4, 5]
    /// ```
    public mutating func appendFrom[I](iterable: I) where I: Iterable, I.Item = T {
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
        self.makeUnique();
        self.grow(myLen + 1);
        var s = self.storage.getValue();
        // Shift elements right
        var i: Int64 = s.len;
        while i > index {
            s.ptr.offset(by: i).write(s.ptr.offset(by: i - 1).read());
            i = i - 1
        }
        s.ptr.offset(by: index).write(element);
        s.len = s.len + 1;
        self.storage.setValue(s)
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
            var s = self.storage.getValue();
            s.len = s.len - 1;
            let value = s.ptr.offset(by: s.len).read();
            self.storage.setValue(s);
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
        var s = self.storage.getValue();
        let removed = s.ptr.offset(by: index).read();
        // Shift elements left
        var i: Int64 = index;
        while i < s.len - 1 {
            s.ptr.offset(by: i).write(s.ptr.offset(by: i + 1).read());
            i = i + 1
        }
        s.len = s.len - 1;
        self.storage.setValue(s);
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
    public mutating func removeSubrange(range: Range[Int64]) {
        let start = range.start;
        let end = range.end;
        let myLen = self.len();
        if start < 0 or end > myLen or start > end {
            fatalError("Array.removeSubrange: range out of bounds")
        }
        let removeCount = end - start;
        if removeCount == 0 {
            return
        }
        self.makeUnique();
        var s = self.storage.getValue();
        // Shift elements left
        var i = start;
        while i < myLen - removeCount {
            s.ptr.offset(by: i).write(s.ptr.offset(by: i + removeCount).read());
            i = i + 1
        }
        s.len = s.len - removeCount;
        self.storage.setValue(s)
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
        var s = self.storage.getValue();
        s.len = 0;
        self.storage.setValue(s)
    }

    /// Keeps only elements for which `predicate` returns true; removes
    /// the rest in place.
    ///
    /// O(n), single pass, stable (relative order preserved). The mirror
    /// operation is `removeAll(matching:)`. For a copy instead of an
    /// in-place edit, use `iter().filter(...).collect()`.
    ///
    /// # Examples
    ///
    /// ```
    /// var arr = [1, 2, 3, 4, 5];
    /// arr.retain(matching: { (x) in x % 2 == 0 });  // [2, 4]
    /// ```
    public mutating func retain(matching predicate: (T) -> Bool) {
        self.makeUnique();
        var s = self.storage.getValue();
        var writeIdx: Int64 = 0;
        for readIdx in 0..<s.len {
            let element = s.ptr.offset(by: readIdx).read();
            if predicate(element) {
                if writeIdx != readIdx {
                    s.ptr.offset(by: writeIdx).write(element)
                }
                writeIdx = writeIdx + 1
            }
        }
        s.len = writeIdx;
        self.storage.setValue(s)
    }

    /// Removes every element for which `predicate` returns true.
    ///
    /// The inverse of `retain(matching:)` — implemented as
    /// `retain` over the negated predicate. O(n), stable.
    ///
    /// # Examples
    ///
    /// ```
    /// var arr = [1, 2, 3, 4, 5];
    /// arr.removeAll(matching: { (x) in x % 2 == 0 });  // [1, 3, 5]
    ///
    /// var names = ["Alice", "", "Bob", ""];
    /// names.removeAll(matching: { (s) in s.isEmpty });  // ["Alice", "Bob"]
    /// ```
    public mutating func removeAll(matching predicate: (T) -> Bool) {
        self.retain(matching: { (x) in predicate(x) == false })
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
        let temp = ptr.offset(by: i).read();
        ptr.offset(by: i).write(ptr.offset(by: j).read());
        ptr.offset(by: j).write(temp)
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
        self.makeUnique();
        var s = self.storage.getValue();
        var left: Int64 = 0;
        var right: Int64 = s.len - 1;
        while left < right {
            let temp = s.ptr.offset(by: left).read();
            s.ptr.offset(by: left).write(s.ptr.offset(by: right).read());
            s.ptr.offset(by: right).write(temp);
            left = left + 1;
            right = right - 1
        }
        self.storage.setValue(s)
    }

    /// Returns a new array with the elements in reverse order.
    ///
    /// Non-mutating. Internally clones via COW (cheap until the next
    /// mutation) then `reverse()`s the copy. Use `reverse()` if you
    /// don't need to keep the original ordering.
    ///
    /// # Examples
    ///
    /// ```
    /// let arr = [1, 2, 3];
    /// let rev = arr.reversed();  // [3, 2, 1]
    /// // arr is still [1, 2, 3]
    /// ```
    public func reversed() -> Array[T] {
        var result = self.clone();
        result.reverse();
        result
    }

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
        let myLen = self.len();
        if myLen <= 1 {
            return
        }
        var normalized = amount % myLen;
        if normalized < 0 {
            normalized = normalized + myLen
        }
        if normalized == 0 {
            return
        }
        // Three-reversal algorithm
        self.makeUnique();
        // Reverse first part [0, normalized)
        self.reverseRange(from: 0, to: normalized);
        // Reverse second part [normalized, len)
        self.reverseRange(from: normalized, to: myLen);
        // Reverse entire array
        self.reverse()
    }

    /// Reverses the half-open sub-range `[start, end)` in place.
    ///
    /// Internal helper used by `rotate(by:)`'s three-reversal algorithm.
    /// Does not bounds-check; callers must pass valid indices.
    private mutating func reverseRange(from start: Int64, to end: Int64) {
        var left = start;
        var right = end - 1;
        let ptr = self.ptr();
        while left < right {
            let temp = ptr.offset(by: left).read();
            ptr.offset(by: left).write(ptr.offset(by: right).read());
            ptr.offset(by: right).write(temp);
            left = left + 1;
            right = right - 1
        }
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
    public mutating func replaceSubrange(range: Range[Int64], with replacement: Array[T]) {
        let start = range.start;
        let end = range.end;
        let myLen = self.len();
        if start < 0 or end > myLen or start > end {
            fatalError("Array.replaceSubrange: range out of bounds")
        }

        let removeCount = end - start;
        let insertCount = replacement.count;
        let newLen = myLen - removeCount + insertCount;

        self.grow(newLen);
        self.makeUnique();
        var s = self.storage.getValue();

        if insertCount > removeCount {
            // Shift elements right
            var i = myLen - 1;
            while i >= end {
                s.ptr.offset(by: i + insertCount - removeCount).write(s.ptr.offset(by: i).read());
                i = i - 1
            }
        } else if insertCount < removeCount {
            // Shift elements left
            var i = end;
            while i < myLen {
                s.ptr.offset(by: start + insertCount + (i - end)).write(s.ptr.offset(by: i).read());
                i = i + 1
            }
        }

        // Copy replacement
        for i in 0..<insertCount {
            s.ptr.offset(by: start + i).write(replacement(unchecked: i))
        }

        s.len = newLen;
        self.storage.setValue(s)
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
    public mutating func shuffle[R](using rng: R) where R: RandomNumberGenerator {
        let n = self.len();
        if n <= 1 {
            return
        }
        self.makeUnique();
        var s = self.storage.getValue();
        var generator = rng;

        // Fisher-Yates shuffle
        var i: Int64 = n - 1;
        while i > 0 {
            // Inline nextInt(below:) since extension methods may not be visible on generic R
            let bound = UInt64(from: i) + 1;
            let rngValue = generator.nextUInt64();
            let j = Int64(from: rngValue.modulo(bound));
            // Swap elements at i and j
            let temp = s.ptr.offset(by: i).read();
            s.ptr.offset(by: i).write(s.ptr.offset(by: j).read());
            s.ptr.offset(by: j).write(temp);
            i = i - 1
        }

        self.storage.setValue(s)
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
        var rng = Lcg64();
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
    public func shuffled[R](using rng: R) -> Array[T] where R: RandomNumberGenerator {
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
    public func shuffled() -> Array[T] {
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
                var s = self.storage.getValue();
                let layout = Layout.array[T](myCap);
                var allocator = SystemAllocator();
                allocator.deallocate(s.ptr.asRaw(), layout);
                s.ptr = Pointer[T].nullPointer();
                s.cap = 0;
                self.storage.setValue(s)
            }
            return
        }

        self.makeUnique();

        // Reallocate to exact size
        let newLayout = Layout.array[T](myLen);
        var allocator = SystemAllocator();
        let result = allocator.allocate(newLayout);
        if let .Some(rawPtr) = result {
            let newPtr = rawPtr.cast[T]();
            let oldStorage = self.storage.getValue();
            for i in 0..<myLen {
                newPtr.offset(by: i).write(oldStorage.ptr.offset(by: i).read())
            }
            if myCap > 0 {
                let oldLayout = Layout.array[T](myCap);
                allocator.deallocate(oldStorage.ptr.asRaw(), oldLayout)
            }
            self.storage.setValue(ArrayStorage(ptr: newPtr, len: myLen, cap: myLen))
        }
    }

    // ========================================================================
    // ACCESSORS (continued)
    // ========================================================================

    /// Returns the first element, or `None` if the array is empty.
    ///
    /// O(1). Read-only — to remove the first element use `popFirst()`.
    /// To find the first element matching a predicate, see
    /// `first(matching:)`.
    ///
    /// # Examples
    ///
    /// ```
    /// [1, 2, 3].first();  // Some(1)
    /// [].first();         // None
    /// ```
    public func first() -> T? {
        if self.len() > 0 {
            .Some(self.ptr().read())
        } else {
            .None
        }
    }

    /// Returns the last element, or `None` if the array is empty.
    ///
    /// O(1). Read-only — to remove the last element use `pop()`. To find
    /// the last element matching a predicate, see `last(matching:)`.
    ///
    /// # Examples
    ///
    /// ```
    /// [1, 2, 3].last();  // Some(3)
    /// [].last();         // None
    /// ```
    public func last() -> T? {
        let myLen = self.len();
        if myLen > 0 {
            .Some(self.ptr().offset(by: myLen - 1).read())
        } else {
            .None
        }
    }

    // ========================================================================
    // ITERATION
    // ========================================================================

    /// Returns a forward iterator over the array's elements.
    ///
    /// The returned `ArrayIterator[T]` aliases the array's buffer; do
    /// not mutate the array while iterating. For grouped views see
    /// `chunks(of:)` and `windows(of:)`.
    ///
    /// # Examples
    ///
    /// ```
    /// for item in arr.iter() { ... }
    /// let doubled = arr.iter().map { (x) in x * 2 }.collect();
    /// ```
    public func iter() -> ArrayIterator[T] {
        ArrayIterator(ptr: self.ptr(), remaining: self.len())
    }

    // ========================================================================
    // SEARCHING
    // ========================================================================

    /// Returns the index of the first element matching `predicate`, or
    /// `None`.
    ///
    /// Linear scan from the front; short-circuits on the first match.
    /// To get the element instead of the index, use `first(matching:)`.
    /// For value-based search on `Equatable` arrays, use
    /// `firstIndex(of:)`.
    ///
    /// # Examples
    ///
    /// ```
    /// let arr = [1, 2, 3, 4, 5];
    /// arr.firstIndex(matching: { (x) in x > 3 });   // Some(3)
    /// arr.firstIndex(matching: { (x) in x > 10 });  // None
    /// ```
    public func firstIndex(matching predicate: (T) -> Bool) -> Int64? {
        let myLen = self.len();
        let myPtr = self.ptr();
        for i in 0..<myLen {
            if predicate(myPtr.offset(by: i).read()) {
                return .Some(i)
            }
        }
        .None
    }

    /// Returns the index of the last element matching `predicate`, or
    /// `None`.
    ///
    /// Linear scan from the back; short-circuits on the first match. The
    /// mirror of `firstIndex(matching:)`. For value-based search on
    /// `Equatable` arrays, use `lastIndex(of:)`.
    ///
    /// # Examples
    ///
    /// ```
    /// let arr = [1, 2, 3, 2, 1];
    /// arr.lastIndex(matching: { (x) in x == 2 });   // Some(3)
    /// arr.lastIndex(matching: { (x) in x == 99 });  // None
    /// ```
    public func lastIndex(matching predicate: (T) -> Bool) -> Int64? {
        let myLen = self.len();
        if myLen == 0 {
            return .None
        }
        let myPtr = self.ptr();
        var i = myLen - 1;
        while i >= 0 {
            if predicate(myPtr.offset(by: i).read()) {
                return .Some(i)
            }
            i = i - 1
        }
        .None
    }

    /// Returns the first element matching `predicate`, or `None`.
    ///
    /// Wraps `firstIndex(matching:)` and reads the element at the
    /// returned index. For just the index, use `firstIndex(matching:)`.
    ///
    /// # Examples
    ///
    /// ```
    /// let arr = [1, 2, 3, 4, 5];
    /// arr.first(matching: { (x) in x > 3 });   // Some(4)
    /// arr.first(matching: { (x) in x > 99 });  // None
    /// ```
    public func first(matching predicate: (T) -> Bool) -> T? {
        if let .Some(idx) = self.firstIndex(matching: predicate) {
            .Some(self(unchecked: idx))
        } else {
            .None
        }
    }

    /// Returns the last element matching `predicate`, or `None`.
    ///
    /// Wraps `lastIndex(matching:)`. For just the index, use
    /// `lastIndex(matching:)`.
    ///
    /// # Examples
    ///
    /// ```
    /// let arr = [1, 2, 3, 2, 1];
    /// arr.last(matching: { (x) in x > 1 });  // Some(2) — the second 2
    /// ```
    public func last(matching predicate: (T) -> Bool) -> T? {
        if let .Some(idx) = self.lastIndex(matching: predicate) {
            .Some(self(unchecked: idx))
        } else {
            .None
        }
    }

    // ========================================================================
    // PREDICATES
    // ========================================================================

    /// `true` when every element satisfies `predicate` (vacuously true
    /// for an empty array).
    ///
    /// Short-circuits on the first failure. The dual is
    /// `any(matching:)`.
    ///
    /// # Examples
    ///
    /// ```
    /// [2, 4, 6].all(matching: { (x) in x % 2 == 0 });  // true
    /// [2, 3, 6].all(matching: { (x) in x % 2 == 0 });  // false
    /// [].all(matching: { (x) in false });              // true (vacuous)
    /// ```
    public func all(matching predicate: (T) -> Bool) -> Bool {
        let myLen = self.len();
        let myPtr = self.ptr();
        for i in 0..<myLen {
            if predicate(myPtr.offset(by: i).read()) == false {
                return false
            }
        }
        true
    }

    /// `true` when at least one element satisfies `predicate` (always
    /// `false` for an empty array).
    ///
    /// Short-circuits on the first match. The dual is `all(matching:)`.
    ///
    /// # Examples
    ///
    /// ```
    /// [1, 2, 3].any(matching: { (x) in x > 2 });  // true
    /// [1, 2, 3].any(matching: { (x) in x > 5 });  // false
    /// [].any(matching: { (x) in true });          // false (empty)
    /// ```
    public func any(matching predicate: (T) -> Bool) -> Bool {
        let myLen = self.len();
        let myPtr = self.ptr();
        for i in 0..<myLen {
            if predicate(myPtr.offset(by: i).read()) {
                return true
            }
        }
        false
    }

    /// Returns the number of elements for which `predicate` is true.
    ///
    /// Linear scan, no short-circuit. For just a presence check use
    /// `any(matching:)`; for a yes/no on every element,
    /// `all(matching:)`.
    ///
    /// # Examples
    ///
    /// ```
    /// [1, 2, 3, 4, 5].countItems(matching: { (x) in x % 2 == 0 });  // 2
    /// [].countItems(matching: { (x) in true });                     // 0
    /// ```
    public func countItems(matching predicate: (T) -> Bool) -> Int64 {
        let myLen = self.len();
        let myPtr = self.ptr();
        var result: Int64 = 0;
        for i in 0..<myLen {
            if predicate(myPtr.offset(by: i).read()) {
                result = result + 1
            }
        }
        result
    }

    // ========================================================================
    // SLICING
    // ========================================================================

    /// Returns a `Slice[T]` over the first `count` elements.
    ///
    /// Borrows the array's buffer; reallocation invalidates it. Pair
    /// with `drop(first:)` to get the complementary suffix. For the
    /// trailing elements, see `suffix(count:)`.
    ///
    /// # Errors
    ///
    /// Panics with `"Array.prefix: count exceeds array length"` if
    /// `count > self.count`.
    ///
    /// # Examples
    ///
    /// ```
    /// [1, 2, 3, 4, 5].prefix(3);  // Slice[1, 2, 3]
    /// [1, 2].prefix(0);           // empty Slice
    /// [1, 2].prefix(9);           // PANIC
    /// ```
    public func prefix(count: Int64) -> Slice[T] {
        let myLen = self.len();
        if count > myLen {
            fatalError("Array.prefix: count exceeds array length")
        }
        Slice(pointer: self.ptr(), count: count)
    }

    /// Returns a `Slice[T]` over the last `count` elements.
    ///
    /// Mirror of `prefix(count:)`. Borrows the array's buffer.
    ///
    /// # Errors
    ///
    /// Panics with `"Array.suffix: count exceeds array length"` if
    /// `count > self.count`.
    ///
    /// # Examples
    ///
    /// ```
    /// [1, 2, 3, 4, 5].suffix(2);  // Slice[4, 5]
    /// [1, 2].suffix(0);           // empty Slice
    /// ```
    public func suffix(count: Int64) -> Slice[T] {
        let myLen = self.len();
        if count > myLen {
            fatalError("Array.suffix: count exceeds array length")
        }
        Slice(pointer: self.ptr().offset(by: myLen - count), count: count)
    }

    /// Returns a `Slice[T]` with the first `count` elements skipped.
    ///
    /// Complement of `prefix(count:)`. Borrows the array's buffer.
    ///
    /// # Errors
    ///
    /// Panics with `"Array.drop(first:): count exceeds array length"` if
    /// `count > self.count`.
    ///
    /// # Examples
    ///
    /// ```
    /// [1, 2, 3, 4, 5].drop(first: 2);  // Slice[3, 4, 5]
    /// [1, 2].drop(first: 2);           // empty Slice
    /// ```
    public func drop(first count: Int64) -> Slice[T] {
        let myLen = self.len();
        if count > myLen {
            fatalError("Array.drop(first:): count exceeds array length")
        }
        Slice(pointer: self.ptr().offset(by: count), count: myLen - count)
    }

    /// Returns a `Slice[T]` with the last `count` elements skipped.
    ///
    /// Complement of `suffix(count:)`. Borrows the array's buffer.
    ///
    /// # Errors
    ///
    /// Panics with `"Array.drop(last:): count exceeds array length"` if
    /// `count > self.count`.
    ///
    /// # Examples
    ///
    /// ```
    /// [1, 2, 3, 4, 5].drop(last: 2);  // Slice[1, 2, 3]
    /// [1, 2].drop(last: 2);           // empty Slice
    /// ```
    public func drop(last count: Int64) -> Slice[T] {
        let myLen = self.len();
        if count > myLen {
            fatalError("Array.drop(last:): count exceeds array length")
        }
        Slice(pointer: self.ptr(), count: myLen - count)
    }

    // ========================================================================
    // CHUNKING
    // ========================================================================

    /// Returns a `ChunksIterator[T]` over non-overlapping `size`-sized
    /// `Slice[T]`s.
    ///
    /// The final chunk may be shorter when `count` is not divisible by
    /// `size`. For overlapping fixed-size views, use `windows(of:)`. The
    /// produced iterator borrows the array's buffer.
    ///
    /// # Errors
    ///
    /// Panics with `"Array.chunks: size must be positive"` if `size <= 0`.
    ///
    /// # Examples
    ///
    /// ```
    /// let arr = [1, 2, 3, 4, 5];
    /// for chunk in arr.chunks(of: 2) {
    ///     // yields Slice[1,2], Slice[3,4], Slice[5]
    /// }
    /// arr.chunks(of: 0);  // PANIC
    /// ```
    public func chunks(of size: Int64) -> ChunksIterator[T] {
        if size <= 0 {
            fatalError("Array.chunks: size must be positive")
        }
        ChunksIterator(ptr: self.ptr(), remaining: self.len(), chunkSize: size)
    }

    /// Returns a `WindowsIterator[T]` over overlapping `size`-sized
    /// `Slice[T]`s.
    ///
    /// Adjacent windows overlap by `size - 1` elements. For
    /// non-overlapping fixed-size groups, use `chunks(of:)`. The
    /// produced iterator borrows the array's buffer.
    ///
    /// # Errors
    ///
    /// Panics with `"Array.windows: size must be positive"` if
    /// `size <= 0`, or `"Array.windows: size exceeds array length"` if
    /// `size > count`.
    ///
    /// # Examples
    ///
    /// ```
    /// let arr = [1, 2, 3, 4];
    /// for window in arr.windows(of: 2) {
    ///     // yields Slice[1,2], Slice[2,3], Slice[3,4]
    /// }
    /// [1, 2].windows(of: 5);  // PANIC: size exceeds array length
    /// ```
    public func windows(of size: Int64) -> WindowsIterator[T] {
        if size <= 0 {
            fatalError("Array.windows: size must be positive")
        }
        if size > self.len() {
            fatalError("Array.windows: size exceeds array length")
        }
        WindowsIterator(ptr: self.ptr(), totalCount: self.len(), windowSize: size)
    }

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
        self.makeUnique();
        var s = self.storage.getValue();
        var lo: Int64 = 0;
        var hi: Int64 = s.len - 1;

        while true {
            // Find first element that doesn't satisfy predicate
            while lo < s.len and predicate(s.ptr.offset(by: lo).read()) {
                lo = lo + 1
            }
            // Find last element that satisfies predicate
            while hi >= 0 and predicate(s.ptr.offset(by: hi).read()) == false {
                hi = hi - 1
            }

            if lo >= hi {
                break
            }

            // Swap
            let temp = s.ptr.offset(by: lo).read();
            s.ptr.offset(by: lo).write(s.ptr.offset(by: hi).read());
            s.ptr.offset(by: hi).write(temp);
            lo = lo + 1;
            hi = hi - 1
        }

        self.storage.setValue(s);
        lo
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
    public func partitioned(by predicate: (T) -> Bool) -> (Array[T], Array[T]) {
        var matching = Array[T]();
        var notMatching = Array[T]();
        let myLen = self.len();
        let myPtr = self.ptr();
        for i in 0..<myLen {
            let element = myPtr.offset(by: i).read();
            if predicate(element) {
                matching.append( element)
            } else {
                notMatching.append( element)
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
    /// O(1) — just bumps the storage `RcBox`'s refcount. The first
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
    public func clone() -> Array[T] {
        Array(storage: self.storage.clone())
    }
}

// ============================================================================
// ARRAY INDEX CONFORMANCES
// ============================================================================

// `Int64` indexes a single element of `Array[T]`.
extend Int64: ArrayIndex[T] {
    type ArrayYield = T

    public func readArray(from array: Array[T]) -> T {
        if self < 0 or self >= array.len() {
            fatalError("Array index out of bounds")
        }
        array.ptr().offset(by: self).read()
    }

    public func readArrayChecked(from array: Array[T]) -> T? {
        if self >= 0 and self < array.len() {
            .Some(array.ptr().offset(by: self).read())
        } else {
            .None
        }
    }

    public func readArrayUnchecked(from array: Array[T]) -> T {
        array.ptr().offset(by: self).read()
    }

    public func writeArray(mutating to array: Array[T], with value: T) {
        if self < 0 or self >= array.len() {
            fatalError("Array index out of bounds")
        }
        array.makeUnique();
        array.ptr().offset(by: self).write(value)
    }

    public func writeArrayUnchecked(mutating to array: Array[T], with value: T) {
        array.makeUnique();
        array.ptr().offset(by: self).write(value)
    }
}

// `Int64` clamping — saturate to `[0, count)`, returning `T?` so the
// empty-array case can surface as `None`.
extend Int64: ArrayClampable[T] {
    type ArrayClampedYield = T?

    public func readArrayClamped(from array: Array[T]) -> T? {
        let len = array.len();
        if len == 0 {
            return .None
        }
        var idx = self;
        if idx < 0 { idx = 0 }
        if idx >= len { idx = len - 1 }
        .Some(array.ptr().offset(by: idx).read())
    }

    public func writeArrayClamped(mutating to array: Array[T], with value: T?) {
        if let .Some(v) = value {
            let len = array.len();
            if len == 0 {
                return
            }
            var idx = self;
            if idx < 0 { idx = 0 }
            if idx >= len { idx = len - 1 }
            array.makeUnique();
            array.ptr().offset(by: idx).write(v)
        }
    }
}

// `Int64` wrapping — modular indexing. Empty-array case is `None`.
extend Int64: ArrayWrappable[T] {
    type ArrayWrappedYield = T?

    public func readArrayWrapped(from array: Array[T]) -> T? {
        let len = array.len();
        if len == 0 {
            return .None
        }
        var idx = self % len;
        if idx < 0 { idx = idx + len }
        .Some(array.ptr().offset(by: idx).read())
    }

    public func writeArrayWrapped(mutating to array: Array[T], with value: T?) {
        if let .Some(v) = value {
            let len = array.len();
            if len == 0 {
                return
            }
            var idx = self % len;
            if idx < 0 { idx = idx + len }
            array.makeUnique();
            array.ptr().offset(by: idx).write(v)
        }
    }
}

// `Range[Int64]` indexes a contiguous half-open slice of `Array[T]`.
//
// Reads return a `Slice[T]` aliasing the array's buffer; reallocation
// invalidates it. Writes copy elements from the source slice into the
// range one-by-one and panic on length mismatch.
extend Range[Int64]: ArrayIndex[T] {
    type ArrayYield = Slice[T]

    public func readArray(from array: Array[T]) -> Slice[T] {
        let start = self.start;
        let end = self.end;
        if start < 0 or end > array.len() or start > end {
            fatalError("Array range out of bounds")
        }
        Slice(pointer: array.ptr().offset(by: start), count: end - start)
    }

    public func readArrayChecked(from array: Array[T]) -> Slice[T]? {
        let start = self.start;
        let end = self.end;
        if start >= 0 and end <= array.len() and start <= end {
            .Some(Slice(pointer: array.ptr().offset(by: start), count: end - start))
        } else {
            .None
        }
    }

    public func readArrayUnchecked(from array: Array[T]) -> Slice[T] {
        Slice(pointer: array.ptr().offset(by: self.start), count: self.end - self.start)
    }

    public func writeArray(mutating to array: Array[T], with value: Slice[T]) {
        let start = self.start;
        let end = self.end;
        if start < 0 or end > array.len() or start > end {
            fatalError("Array range out of bounds")
        }
        let rangeLen = end - start;
        if value.count != rangeLen {
            fatalError("Slice length doesn't match range length")
        }
        array.makeUnique();
        var i = 0;
        while i < rangeLen {
            array.ptr().offset(by: start + i).write(value.pointer.offset(by: i).read());
            i = i + 1;
        }
    }

    public func writeArrayUnchecked(mutating to array: Array[T], with value: Slice[T]) {
        let start = self.start;
        let rangeLen = self.end - start;
        if value.count != rangeLen {
            fatalError("Slice length doesn't match range length")
        }
        array.makeUnique();
        var i = 0;
        while i < rangeLen {
            array.ptr().offset(by: start + i).write(value.pointer.offset(by: i).read());
            i = i + 1;
        }
    }
}

// `Range[Int64]` clamping — both endpoints clamp into `[0, count]`,
// always producing a (possibly empty) `Slice[T]`. Writes panic on
// length mismatch after clamping.
extend Range[Int64]: ArrayClampable[T] {
    type ArrayClampedYield = Slice[T]

    public func readArrayClamped(from array: Array[T]) -> Slice[T] {
        let len = array.len();
        var start = self.start;
        var end = self.end;
        if start < 0 { start = 0 }
        if end > len { end = len }
        if start > end { start = end }
        Slice(pointer: array.ptr().offset(by: start), count: end - start)
    }

    public func writeArrayClamped(mutating to array: Array[T], with value: Slice[T]) {
        let len = array.len();
        var start = self.start;
        var end = self.end;
        if start < 0 { start = 0 }
        if end > len { end = len }
        if start > end { start = end }
        let rangeLen = end - start;
        if value.count != rangeLen {
            fatalError("Slice length doesn't match clamped range length")
        }
        array.makeUnique();
        var i = 0;
        while i < rangeLen {
            array.ptr().offset(by: start + i).write(value.pointer.offset(by: i).read());
            i = i + 1;
        }
    }
}

// `ClosedRange[Int64]` indexes a contiguous closed slice of `Array[T]` —
// both endpoints are included. Internally converts to `[start, end+1)`
// so the rest of the implementation matches `Range`.
extend ClosedRange[Int64]: ArrayIndex[T] {
    type ArrayYield = Slice[T]

    public func readArray(from array: Array[T]) -> Slice[T] {
        let start = self.start;
        let endExclusive = self.end + 1;
        if start < 0 or endExclusive > array.len() or start > endExclusive {
            fatalError("Array range out of bounds")
        }
        Slice(pointer: array.ptr().offset(by: start), count: endExclusive - start)
    }

    public func readArrayChecked(from array: Array[T]) -> Slice[T]? {
        let start = self.start;
        let endExclusive = self.end + 1;
        if start >= 0 and endExclusive <= array.len() and start <= endExclusive {
            .Some(Slice(pointer: array.ptr().offset(by: start), count: endExclusive - start))
        } else {
            .None
        }
    }

    public func readArrayUnchecked(from array: Array[T]) -> Slice[T] {
        let start = self.start;
        let endExclusive = self.end + 1;
        Slice(pointer: array.ptr().offset(by: start), count: endExclusive - start)
    }

    public func writeArray(mutating to array: Array[T], with value: Slice[T]) {
        let start = self.start;
        let endExclusive = self.end + 1;
        if start < 0 or endExclusive > array.len() or start > endExclusive {
            fatalError("Array range out of bounds")
        }
        let rangeLen = endExclusive - start;
        if value.count != rangeLen {
            fatalError("Slice length doesn't match range length")
        }
        array.makeUnique();
        var i = 0;
        while i < rangeLen {
            array.ptr().offset(by: start + i).write(value.pointer.offset(by: i).read());
            i = i + 1;
        }
    }

    public func writeArrayUnchecked(mutating to array: Array[T], with value: Slice[T]) {
        let start = self.start;
        let rangeLen = self.end + 1 - start;
        if value.count != rangeLen {
            fatalError("Slice length doesn't match range length")
        }
        array.makeUnique();
        var i = 0;
        while i < rangeLen {
            array.ptr().offset(by: start + i).write(value.pointer.offset(by: i).read());
            i = i + 1;
        }
    }
}

// ============================================================================
// CONDITIONAL EXTENSIONS
// ============================================================================

/// `Equatable` and value-based search/dedup operations available when the
/// element type itself is `Equatable`.
extend Array[T]: Equatable where T: Equatable {
    /// Element-wise equality: arrays are equal iff they have the same
    /// `count` and every corresponding pair of elements is equal.
    ///
    /// Short-circuits on the first mismatch. Order matters —
    /// `[1, 2, 3]` is not equal to `[3, 2, 1]`.
    ///
    /// # Examples
    ///
    /// ```
    /// [1, 2, 3].equals([1, 2, 3]);  // true
    /// [1, 2, 3].equals([1, 2]);     // false
    /// [1, 2, 3].equals([3, 2, 1]);  // false
    /// ```
    public func equals(other: Array[T]) -> Bool {
        let selfCount = self.count;
        let otherCount = other.count;
        if selfCount != otherCount {
            return false
        }
        var i: Int64 = 0;
        var equal: Bool = true;
        while i < selfCount and equal {
            if self(unchecked: i).equals(other(unchecked: i)) == false {
                equal = false
            }
            i = i + 1
        }
        equal
    }

    /// `true` if the array contains an element equal to `element`.
    ///
    /// Linear scan; short-circuits on the first match. For predicate-
    /// based searching see `any(matching:)` or `firstIndex(matching:)`.
    ///
    /// # Examples
    ///
    /// ```
    /// [1, 2, 3].contains(2);  // true
    /// [1, 2, 3].contains(5);  // false
    /// ```
    public func contains(element: T) -> Bool {
        self.firstIndex(matching: { (x) in x.equals(element) }).isSome()
    }

    /// Returns the index of the first element equal to `element`, or
    /// `None`.
    ///
    /// Wraps `firstIndex(matching:)` with `equals(element)`. The mirror
    /// is `lastIndex(of:)`.
    ///
    /// # Examples
    ///
    /// ```
    /// [1, 2, 3, 2].firstIndex(of: 2);  // Some(1)
    /// [1, 2, 3].firstIndex(of: 5);     // None
    /// ```
    public func firstIndex(of element: T) -> Int64? {
        self.firstIndex(matching: { (x) in x.equals(element) })
    }

    /// Returns the index of the last element equal to `element`, or
    /// `None`.
    ///
    /// Wraps `lastIndex(matching:)` with `equals(element)`. The mirror
    /// is `firstIndex(of:)`.
    ///
    /// # Examples
    ///
    /// ```
    /// [1, 2, 3, 2].lastIndex(of: 2);  // Some(3)
    /// [1, 2, 3].lastIndex(of: 5);     // None
    /// ```
    public func lastIndex(of element: T) -> Int64? {
        self.lastIndex(matching: { (x) in x.equals(element) })
    }

    /// `true` if the array's leading elements match `prefix` exactly.
    ///
    /// An empty prefix always matches; a prefix longer than the array
    /// never matches. Mirror of `ends(with:)`.
    ///
    /// # Examples
    ///
    /// ```
    /// [1, 2, 3].starts(with: [1, 2]);     // true
    /// [1, 2, 3].starts(with: [1, 2, 3]);  // true (full match)
    /// [1, 2, 3].starts(with: [2, 3]);     // false
    /// [1, 2].starts(with: [1, 2, 3]);     // false (prefix longer)
    /// [1, 2, 3].starts(with: []);         // true (vacuous)
    /// ```
    public func starts(with prefix: Array[T]) -> Bool {
        let prefixLen = prefix.count;
        if prefixLen > self.count {
            return false
        }
        for i in 0..<prefixLen {
            if self(unchecked: i).equals(prefix(unchecked: i)) == false {
                return false
            }
        }
        true
    }

    /// `true` if the array's trailing elements match `suffix` exactly.
    ///
    /// An empty suffix always matches; a suffix longer than the array
    /// never matches. Mirror of `starts(with:)`.
    ///
    /// # Examples
    ///
    /// ```
    /// [1, 2, 3].ends(with: [2, 3]);  // true
    /// [1, 2, 3].ends(with: [1, 2]);  // false
    /// [1, 2, 3].ends(with: []);      // true (vacuous)
    /// ```
    public func ends(with suffix: Array[T]) -> Bool {
        let suffixLen = suffix.count;
        let myLen = self.count;
        if suffixLen > myLen {
            return false
        }
        let offset = myLen - suffixLen;
        for i in 0..<suffixLen {
            if self(unchecked: offset + i).equals(suffix(unchecked: i)) == false {
                return false
            }
        }
        true
    }

    /// Splits the array on each element equal to `separator`, returning
    /// the in-between runs as `Slice[T]`s.
    ///
    /// Separators themselves are dropped, but empty runs (between
    /// adjacent separators, or before the first / after the last) are
    /// preserved as empty slices. The result therefore always has length
    /// `(separatorCount + 1)`. The slices alias the source buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// [1, 0, 2, 0, 3].split(0);
    /// // [Slice[1], Slice[2], Slice[3]]
    ///
    /// [0, 1, 0, 0, 2, 0].split(0);
    /// // [Slice[], Slice[1], Slice[], Slice[2], Slice[]]
    ///
    /// [1, 2, 3].split(0);
    /// // [Slice[1, 2, 3]] — separator not found
    ///
    /// [].split(0);
    /// // [Slice[]] — empty array yields one empty slice
    /// ```
    public func split(separator: T) -> Array[Slice[T]] {
        var result = Array[Slice[T]]();
        let myLen = self.count;
        var start: Int64 = 0;
        for i in 0..<myLen {
            if self(unchecked: i).equals(separator) {
                result.append( Slice(pointer: self.asPointer().offset(by: start), count: i - start));
                start = i + 1
            }
        }
        result.append( Slice(pointer: self.asPointer().offset(by: start), count: myLen - start));
        result
    }

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
        if let .Some(idx) = self.firstIndex(matching: { (x) in x.equals(element) }) {
            let _ = self.remove(at: idx);
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
        self.retain(matching: { (x) in x.equals(element) == false })
    }

    /// Removes runs of consecutive equal elements, in place.
    ///
    /// Only adjacent duplicates collapse — non-adjacent equal values are
    /// kept. To deduplicate globally, `sort()` first or, for `Hash`
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
        var s = self.storage.getValue();
        var writeIdx: Int64 = 1;
        for readIdx in 1..<s.len {
            let current = s.ptr.offset(by: readIdx).read();
            let previous = s.ptr.offset(by: writeIdx - 1).read();
            if current.equals(previous) == false {
                if writeIdx != readIdx {
                    s.ptr.offset(by: writeIdx).write(current)
                }
                writeIdx = writeIdx + 1
            }
        }
        s.len = writeIdx;
        self.storage.setValue(s)
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
    public func deduped() -> Array[T] {
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
extend Array[T]: ArrayMatchable {
    /// `ArrayMatchable` element type — what the pattern bindings extract.
    type Element = T

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
        self(unchecked: index)
    }

    /// Pattern-matcher hook returning the half-open `[from, to)` slice.
    ///
    /// Used to bind `..rest` segments. The matcher guarantees the
    /// indices are in range.
    public func matchSlice(from: Int64, to: Int64) -> Slice[T] {
        Slice(pointer: self.asPointer().offset(by: from), count: to - from)
    }
}

// ============================================================================
// COMPARABLE EXTENSION
// ============================================================================

/// Ordering-aware operations available when `T: Comparable`.
extend Array[T] where T: Comparable {
    /// Sorts the array in ascending order using the natural `<` ordering.
    ///
    /// Stable insertion sort under the hood (see the custom-comparator
    /// `sort(by:)` for the algorithm). For descending or custom orderings
    /// pass a comparator to `sort(by:)`. Non-mutating variant: `sorted()`.
    ///
    /// # Examples
    ///
    /// ```
    /// var arr = [3, 1, 4, 1, 5];
    /// arr.sort();  // [1, 1, 3, 4, 5]
    /// ```
    public mutating func sort() {
        self.sort(by: { (a, b) in a < b })
    }

    /// Returns a new array sorted in ascending order; original unchanged.
    ///
    /// Non-mutating mirror of `sort()`. Internally clones via COW then
    /// sorts the copy.
    ///
    /// # Examples
    ///
    /// ```
    /// let arr = [3, 1, 4, 1, 5];
    /// let sorted = arr.sorted();  // [1, 1, 3, 4, 5]
    /// // arr is still [3, 1, 4, 1, 5]
    /// ```
    public func sorted() -> Array[T] {
        self.sorted(by: { (a, b) in a < b })
    }

    /// Returns the smallest element, or `None` if the array is empty.
    ///
    /// Single linear pass; ties go to the first occurrence. Pair with
    /// `max()` for the upper bound.
    ///
    /// # Examples
    ///
    /// ```
    /// [3, 1, 4].min();  // Some(1)
    /// [].min();         // None
    /// ```
    public func min() -> T? {
        if self.count == 0 {
            return .None
        }
        var result = self(unchecked: 0);
        for i in 1..<self.count {
            let element = self(unchecked: i);
            if element < result {
                result = element
            }
        }
        .Some(result)
    }

    /// Returns the largest element, or `None` if the array is empty.
    ///
    /// Single linear pass; ties go to the first occurrence. Pair with
    /// `min()` for the lower bound.
    ///
    /// # Examples
    ///
    /// ```
    /// [3, 1, 4].max();  // Some(4)
    /// [].max();         // None
    /// ```
    public func max() -> T? {
        if self.count == 0 {
            return .None
        }
        var result = self(unchecked: 0);
        for i in 1..<self.count {
            let element = self(unchecked: i);
            if element > result {
                result = element
            }
        }
        .Some(result)
    }

    /// `true` if the array is sorted in non-decreasing (ascending) order.
    ///
    /// Equal adjacent elements are allowed. Empty and single-element
    /// arrays are vacuously sorted. Useful as a precondition for
    /// `binarySearch(element:)`.
    ///
    /// # Examples
    ///
    /// ```
    /// [1, 2, 3].isSorted();  // true
    /// [1, 3, 2].isSorted();  // false
    /// [1, 1, 1].isSorted();  // true (equal adjacents allowed)
    /// [].isSorted();         // true (vacuous)
    /// ```
    public func isSorted() -> Bool {
        if self.count <= 1 {
            return true
        }
        for i in 1..<self.count {
            if self(unchecked: i) < self(unchecked: i - 1) {
                return false
            }
        }
        true
    }

    /// Returns the index of `element` via binary search, or `None`.
    ///
    /// O(log n). When the array contains duplicates, *which* matching
    /// index is returned is unspecified. For unsorted data use
    /// `firstIndex(of:)` instead.
    ///
    /// # Safety
    ///
    /// The array must be sorted in ascending order (per `isSorted()`).
    /// Calling this on an unsorted array does not crash, but the result
    /// is meaningless (false negatives become possible).
    ///
    /// # Examples
    ///
    /// ```
    /// let arr = [1, 2, 3, 4, 5];
    /// arr.binarySearch(3);  // Some(2)
    /// arr.binarySearch(6);  // None
    /// ```
    public func binarySearch(element: T) -> Int64? {
        var lo: Int64 = 0;
        var hi: Int64 = self.count;
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            let midVal = self(unchecked: mid);
            if midVal < element {
                lo = mid + 1
            } else if midVal > element {
                hi = mid
            } else {
                return .Some(mid)
            }
        }
        .None
    }
}

// ============================================================================
// HASH EXTENSION
// ============================================================================

/// Set-like deduplication available when `T: Hash`.
extend Array[T] where T: Hash {
    /// Returns a new array containing each distinct element once, in the
    /// order of first occurrence.
    ///
    /// Currently O(n²) (linear scan per insert). For an O(n) build, push
    /// the elements through a `Set` first. The in-place mirror is
    /// `removeDuplicates()`. Compare with `dedup()`, which only collapses
    /// adjacent duplicates and does not require `Hash`.
    ///
    /// # Examples
    ///
    /// ```
    /// [1, 2, 1, 3, 2, 4].unique();  // [1, 2, 3, 4]
    /// ["a", "a", "b"].unique();      // ["a", "b"]
    /// ```
    public func unique() -> Array[T] {
        var result = Array[T]();
        let myLen = self.count;
        for i in 0..<myLen {
            let element = self(unchecked: i);
            var found = false;
            for j in 0..<result.count {
                if result(unchecked: j).equals(element) {
                    found = true
                }
            }
            if found == false {
                result.append( element)
            }
        }
        result
    }

    /// Removes every duplicate in place, keeping the first occurrence.
    ///
    /// Implemented by replacing storage with the result of `unique()`,
    /// so the same O(n²) caveat applies. The non-mutating mirror is
    /// `unique()`.
    ///
    /// # Examples
    ///
    /// ```
    /// var arr = [1, 2, 1, 3, 2];
    /// arr.removeDuplicates();  // [1, 2, 3]
    /// ```
    public mutating func removeDuplicates() {
        self = self.unique()
    }
}

// ============================================================================
// CUSTOM SORTING EXTENSION
// ============================================================================

/// Custom-comparator and key-extracting sort overloads, available for
/// every element type (no `Comparable` requirement).
extend Array[T] {
    /// Sorts the array in place using a `<`-style comparator.
    ///
    /// The comparator returns `true` when its first argument should come
    /// before the second. Uses insertion sort — O(n²) worst-case but
    /// stable and excellent for small or nearly-sorted inputs. Pass a
    /// reversed comparator for descending order.
    ///
    /// # Examples
    ///
    /// ```
    /// var arr = [1, 5, 3, 2, 4];
    /// arr.sort(by: { (a, b) in a > b });  // [5, 4, 3, 2, 1] descending
    /// ```
    public mutating func sort(by comparator: (T, T) -> Bool) {
        let n = self.count;
        if n <= 1 {
            return
        }
        self.makeUnique();
        // Insertion sort (simple and stable)
        for i in 1..<n {
            let key = self(unchecked: i);
            var j = i - 1;
            while j >= 0 and comparator(key, self(unchecked: j)) {
                self(unchecked: j + 1) = self(unchecked: j);
                j = j - 1
            }
            self(unchecked: j + 1) = key
        }
    }

    /// Returns a new array sorted by a custom comparator. Original
    /// unchanged.
    ///
    /// Non-mutating mirror of `sort(by:)`. Useful for one-shot orderings
    /// such as case-insensitive string sorts.
    ///
    /// # Examples
    ///
    /// ```
    /// let arr = ["apple", "Banana", "cherry"];
    /// let sorted = arr.sorted(by: { (a, b) in a.lowercase() < b.lowercase() });
    /// ```
    public func sorted(by comparator: (T, T) -> Bool) -> Array[T] {
        var result = self.clone();
        result.sort(by: comparator);
        result
    }

    /// Sorts the array in place by an extracted `Comparable` key.
    ///
    /// Equivalent to `sort(by: { (a, b) in key(a) < key(b) })`. The key
    /// closure runs O(n²) times in the worst case (insertion sort), so
    /// keep it cheap. For descending order, pass a comparator to
    /// `sort(by:)` instead.
    ///
    /// # Examples
    ///
    /// ```
    /// var people = [Person("Alice", 30), Person("Bob", 25)];
    /// people.sort(byKey: { (p) in p.age });  // sorted by age ascending
    /// ```
    public mutating func sort[K](byKey key: (T) -> K) where K: Comparable {
        self.sort(by: { (a, b) in key(a) < key(b) })
    }

    /// Returns a new array sorted by an extracted `Comparable` key;
    /// original unchanged.
    ///
    /// Non-mutating mirror of `sort(byKey:)`.
    ///
    /// # Examples
    ///
    /// ```
    /// let words = ["hi", "hello", "hey"];
    /// let byLength = words.sorted(byKey: { (w) in w.count });  // ["hi", "hey", "hello"]
    /// ```
    public func sorted[K](byKey key: (T) -> K) -> Array[T] where K: Comparable {
        self.sorted(by: { (a, b) in key(a) < key(b) })
    }
}

// ============================================================================
// NESTED STRUCTURE EXTENSIONS
// ============================================================================

/// Flattening for arrays whose elements are themselves `Iterable`.
extend Array[T] where T: Iterable {
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
            var iter = self(unchecked: i).iter();
            while let .Some(item) = iter.next() {
                result.append( item)
            }
        }
        result
    }
}

/// String-joining for arrays whose elements are `Formattable`.
extend Array[T] where T: Formattable {
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
        var result = self(unchecked: 0).format();
        for i in 1..<self.count {
            result = result + separator;
            result = result + self(unchecked: i).format()
        }
        result
    }
}

// ============================================================================
// FORMATTABLE CONFORMANCE
// ============================================================================

/// `Formattable` conformance — renders an array as `"[a, b, c]"` when its
/// elements are themselves `Formattable`.
///
/// Drives string interpolation: `"\{[1, 2, 3]}"` produces `"[1, 2, 3]"`.
/// For a flat concatenation without brackets, use `joined(separator:)`.
extend Array[T]: Formattable where T: Formattable {
    /// Renders the array as `"[" + elements.joined(", ") + "]"`, passing
    /// `options` through to each element's `format`.
    ///
    /// Empty arrays render as `"[]"`.
    ///
    /// # Examples
    ///
    /// ```
    /// [1, 2, 3].format();         // "[1, 2, 3]"
    /// Array[Int64]().format();    // "[]"
    /// "\{[1, 2, 3]}";             // "[1, 2, 3]" (via interpolation)
    /// ```
    public func format(options: FormatOptions = FormatOptions.default()) -> String {
        var result = "[";
        let myLen = self.count;
        for i in 0..<myLen {
            if i > 0 {
                result = result + ", "
            }
            result = result + self(unchecked: i).format(options)
        }
        result = result + "]";
        result
    }
}

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
public type ArrayTypeOperator[T] = Array[T];
