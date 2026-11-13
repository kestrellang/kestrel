// Deque[T] - ring-buffer double-ended queue with COW semantics

module std.collections

import std.core.(Bool, Cloneable, Defaultable, fatalError)
import std.numeric.(Int64)
import std.result.(Optional)
import std.memory.(Layout, Pointer, Allocator, GlobalAllocator, CowBox)
import std.collections.(Slice)
import std.iter.(Iterator, Iterable)
import std.ffi.(memcpy)

// ============================================================================
// DEQUE STORAGE (Internal)
// ============================================================================

/// Ring-buffer storage cell for `Deque[T]`.
///
/// Wrapped in a `CowBox` by `Deque[T]` so that copying a deque simply
/// bumps a reference count; mutations write through `storage.value`, which
/// performs the copy-on-write barrier first. The `clone()` method linearizes
/// the ring buffer into a fresh contiguous allocation.
struct DequeStorage[T, A]: Cloneable where A: Allocator {
    /// Heap pointer to the element buffer; null when `cap == 0`.
    var ptr: Pointer[T]
    /// Number of initialized elements in the ring buffer.
    var len: Int64
    /// Total slots allocated; always `>= len`.
    var cap: Int64
    /// Index of the logical first element in the ring buffer.
    var head: Int64
    /// Where the buffer came from and goes back to; a handle, copied with
    /// the storage (see `ArrayStorage.allocator`).
    var allocator: A

    /// @name From Fields
    /// Constructs a `DequeStorage` from raw fields.
    ///
    /// The caller must guarantee `len <= cap`, `head < cap` (or both 0),
    /// and `ptr` valid for `cap` elements when `cap > 0`.
    init(ptr ptr: Pointer[T], len len: Int64, cap cap: Int64, head head: Int64, allocator allocator: A) {
        self.ptr = ptr;
        self.len = len;
        self.cap = cap;
        self.head = head;
        self.allocator = allocator;
    }

    /// Deep-copies the storage, linearizing the ring buffer into a fresh
    /// contiguous allocation sized exactly to `len`.
    ///
    /// The new buffer starts with `head == 0`, so elements are laid out
    /// contiguously regardless of where they wrapped in the source.
    /// Returns an empty storage with a null pointer when `len == 0`.
    /// Panics if allocation fails.
    func clone() -> DequeStorage[T, A] {
        if self.cap == 0 {
            return DequeStorage(
                ptr: Pointer[T].nullPointer(),
                len: 0,
                cap: 0,
                head: 0,
                allocator: self.allocator
            )
        }
        let layout = Layout.array[T](self.cap);
        var allocator = self.allocator;
        let result = allocator.allocate(layout);
        if let .Some(rawPtr) = result {
            let newPtr = rawPtr.cast[T]();
            if Bool(boolLiteral: lang.is_bitwise[T]()) {
                self.copyLinear(to: newPtr);
            } else {
                for i in 0..<self.len {
                    var phys = self.head + i;
                    if phys >= self.cap { phys = phys - self.cap; }
                    unsafe newPtr.offset(by: i).write(self.ptr.offset(by: phys).read());
                }
            }
            DequeStorage(ptr: newPtr, len: self.len, cap: self.cap, head: 0, allocator: allocator)
        } else {
            fatalError("DequeStorage clone allocation failed")
        }
    }

    /// Bit-copies the `len` live elements, in order, to the start of `dest`
    /// (room for `len`): the run from `head`, then the part that wrapped.
    /// That moves them (the source slots must then count as empty) or,
    /// for a byte-copied `T`, copies them.
    func copyLinear(to dest: Pointer[T]) {
        if self.len == 0 { return }
        let size = Layout.of[T]().size;
        let first = if self.head + self.len > self.cap { self.cap - self.head } else { self.len };
        let _ = unsafe memcpy(dest.asRaw(), self.ptr.offset(by: self.head).asRaw(), first * size);
        if first < self.len {
            let _ = unsafe memcpy(dest.offset(by: first).asRaw(), self.ptr.asRaw(), (self.len - first) * size);
        }
    }

    /// Drops every live element, then frees the underlying ring buffer.
    ///
    /// Runs when the last `RcBox` reference to this storage drops (COW
    /// guarantees the buffer is uniquely owned, so each element is dropped
    /// exactly once). The `len` live elements occupy ring positions
    /// `(head + i) % cap`; uninitialized slots are skipped. Skipping these
    /// destructors (the previous behavior) leaked the owned heap of every
    /// non-trivial element, e.g. the `String`s in a `Deque[String]`. For a
    /// trivially-droppable `T` the per-element `dropInPlace` is a no-op.
    deinit {
        if self.cap > 0 {
            var i: Int64 = 0;
            while i < self.len {
                var phys = self.head + i;
                if phys >= self.cap { phys = phys - self.cap; };
                unsafe self.ptr.offset(by: phys).dropInPlace();
                i = i + 1
            };
            let layout = Layout.array[T](self.cap);
            var allocator = self.allocator;
            unsafe allocator.deallocate(self.ptr.asRaw(), layout)
        }
    }
}

// ============================================================================
// DEQUE ITERATOR
// ============================================================================

/// Iterator over a `Deque[T]`, walking the ring buffer from head through
/// `remaining` elements.
///
/// Created by `Deque.iter()`. Yields elements front-to-back, wrapping
/// around the ring buffer boundary transparently.
///
/// # Representation
///
/// Holds a raw pointer into the deque's ring buffer, the buffer
/// capacity, the current physical position, and a remaining-element
/// count. Does not own the storage.
public struct DequeIterator[T]: Iterator {
    /// `Iterator` element type.
    type Item = T

    /// Pointer into the ring buffer (not owned).
    private var ptr: Pointer[T]
    /// Ring buffer capacity (needed for wrap-around arithmetic).
    private var cap: Int64
    /// Current physical index in the ring buffer.
    private var pos: Int64
    /// Number of elements left to yield.
    private var remaining: Int64

    /// @name From Fields
    /// Constructs an iterator from the ring buffer's raw state.
    ///
    /// Called internally by `Deque.iter()`.
    public init(ptr ptr: Pointer[T], cap cap: Int64, pos pos: Int64, remaining remaining: Int64) {
        self.ptr = ptr;
        self.cap = cap;
        self.pos = pos;
        self.remaining = remaining;
    }

    /// Advances the iterator and returns the next element, or `.None`
    /// when all elements have been consumed.
    ///
    /// Handles the ring-buffer wrap-around: when `pos` reaches `cap` it
    /// resets to 0.
    ///
    /// # Examples
    ///
    /// ```
    /// var d = Deque[Int64](from: [10, 20]);
    /// var it = d.iter();
    /// it.next();  // .Some(10)
    /// it.next();  // .Some(20)
    /// it.next();  // .None
    /// ```
    public mutating func next() -> T? {
        if self.remaining <= 0 { return .None; }
        let value = unsafe self.ptr.offset(by: self.pos).read();
        self.pos = self.pos + 1;
        if self.pos >= self.cap { self.pos = 0; }
        self.remaining = self.remaining - 1;
        .Some(value)
    }
}

// ============================================================================
// DEQUE
// ============================================================================

/// A double-ended queue backed by a ring buffer with copy-on-write storage.
///
/// O(1) amortized `append`/`prepend`/`popLast`/`popFirst` and O(1)
/// random access by index. Storage is shared between copies until one
/// mutates, at which point the COW barrier fires.
///
/// # Examples
///
/// ```
/// var d = Deque[Int64]();
/// d.append(1);
/// d.prepend(0);
/// d.append(2);
/// d.popFirst();  // .Some(0)
/// d.popLast();   // .Some(2)
/// ```
///
/// # Representation
///
/// Holds a `CowBox[DequeStorage[T]]`. The storage is a `(ptr, len, cap,
/// head)` quad over a heap-allocated ring buffer.
///
/// # Memory Model
///
/// Reference-counted storage with copy-on-write value semantics via
/// `CowBox`. Copying a `Deque` is O(1); the first mutation on a shared
/// copy triggers a deep clone that linearizes the ring buffer.
///
/// # Guarantees
///
/// - `append`/`prepend` are O(1) amortized; growth is geometric.
/// - `popLast`/`popFirst` are O(1).
/// - Subscript access is O(1).
/// - Iteration order is front-to-back.
public struct Deque[T, A = GlobalAllocator]: Iterable, Cloneable where A: Allocator {
    /// `Iterable` element type.
    type Item = T
    /// `Iterable` iterator type.
    type TargetIterator = DequeIterator[T]

    /// COW-wrapped ring-buffer storage.
    fileprivate var storage: CowBox[DequeStorage[T, A]]

    // -- internal helpers --

    /// Reads the current element count from the COW storage.
    fileprivate func len() -> Int64 { self.storage.value.len }
    /// Reads the current buffer capacity from the COW storage.
    fileprivate func cap() -> Int64 { self.storage.value.cap }
    /// Reads the ring-buffer head offset from the COW storage.
    fileprivate func head() -> Int64 { self.storage.value.head }
    /// Reads the raw element pointer from the COW storage.
    fileprivate func rawPtr() -> Pointer[T] { self.storage.value.ptr }

    // ========================================================================
    // CONSTRUCTORS
    // ========================================================================

    /// @name With Allocator
    /// An empty deque whose buffer will come from `allocator`. Nothing is
    /// allocated until the first push.
    ///
    /// # Examples
    ///
    /// ```
    /// var arena = ArenaAllocator();
    /// var d = Deque[Int64, ArenaAllocator](allocator: arena);
    /// d.append(1);  // allocated from the arena
    /// ```
    public init(allocator allocator: A) {
        self.storage = CowBox(DequeStorage(
            ptr: Pointer[T].nullPointer(),
            len: 0,
            cap: 0,
            head: 0,
            allocator: allocator
        ));
    }

    /// @name With Capacity And Allocator
    /// An empty deque with room for `capacity` elements, allocated from
    /// `allocator`. Panics on allocation failure.
    public init(capacity capacity: Int64, allocator allocator: A) {
        self.init(allocator: allocator);
        self.grow(minCapacity: capacity)
    }

    /// The allocator this deque's buffer comes from.
    public var allocator: A { self.storage.value.allocator }

    // ========================================================================
    // PROPERTIES
    // ========================================================================

    /// Number of elements in the deque.
    ///
    /// # Examples
    ///
    /// ```
    /// let d = Deque[Int64](from: [1, 2, 3]);
    /// d.count;  // 3
    /// ```
    public var count: Int64 { self.len() }

    /// True when the deque contains no elements.
    ///
    /// # Examples
    ///
    /// ```
    /// Deque[Int64]().isEmpty;  // true
    /// Deque[Int64](from: [1]).isEmpty;  // false
    /// ```
    public var isEmpty: Bool { self.len() == 0 }

    /// Number of elements the buffer can hold without reallocating.
    ///
    /// The deque automatically grows when `count` exceeds `capacity`,
    /// so this is mainly useful for pre-sizing via `reserveCapacity()`.
    ///
    /// # Examples
    ///
    /// ```
    /// var d = Deque[Int64](capacity: 16);
    /// d.capacity;  // 16
    /// ```
    public var capacity: Int64 { self.cap() }

    // ========================================================================
    // CAPACITY MANAGEMENT
    // ========================================================================

    /// Grows the ring buffer to hold at least `minCapacity` elements,
    /// linearizing on reallocation so head resets to 0.
    private mutating func grow(minCapacity minCapacity: Int64) {
        let myCap = self.cap();
        if myCap >= minCapacity { return; }

        var newCap = myCap;
        if newCap == 0 { newCap = 4; }
        while newCap < minCapacity { newCap = newCap * 2; }

        let newLayout = Layout.array[T](newCap);
        var allocator = self.storage.value.allocator;
        let result = allocator.allocate(newLayout);
        if let .Some(rawPtr) = result {
            let newPtr = rawPtr.cast[T]();
            // Writing through `value` forks shared storage first. A move is
            // a bit copy, so the ring is linearized into the new buffer with
            // (at most) two memcpys, then the old buffer is freed.
            let s = &mutating self.storage.value;
            s.copyLinear(to: newPtr);
            if s.cap > 0 {
                unsafe allocator.deallocate(s.ptr.asRaw(), Layout.array[T](s.cap))
            }
            s.ptr = newPtr;
            s.cap = newCap;
            s.head = 0
        } else {
            fatalError("Deque grow failed")
        }
    }

    /// Ensures the buffer can hold at least `capacity` elements without
    /// reallocating.
    ///
    /// If the current capacity already meets or exceeds `minimumCapacity`, this
    /// is a no-op. Otherwise the ring buffer is reallocated and
    /// linearized. Useful before a burst of `append`/`prepend` calls
    /// when the final size is known in advance.
    ///
    /// # Examples
    ///
    /// ```
    /// var d = Deque[Int64]();
    /// d.reserveCapacity(100);
    /// ```
    public mutating func reserveCapacity(minimumCapacity: Int64) {
        if minimumCapacity > self.cap() {
            self.grow(minCapacity: minimumCapacity)
        }
    }

    /// Removes all elements, retaining allocated capacity.
    ///
    /// After calling `clear()`, `count` is 0 and `head` resets to 0, but
    /// the buffer stays allocated so subsequent pushes avoid reallocation.
    ///
    /// # Examples
    ///
    /// ```
    /// var d = Deque[Int64](from: [1, 2, 3]);
    /// d.clear();
    /// d.isEmpty;  // true
    /// ```
    public mutating func clear() {
        let s = &mutating self.storage.value;
        // Drop the live elements; resetting `len` alone leaked them.
        for i in 0..<s.len {
            var phys = s.head + i;
            if phys >= s.cap { phys = phys - s.cap; }
            unsafe s.ptr.offset(by: phys).dropInPlace()
        }
        s.len = 0;
        s.head = 0
    }

    // ========================================================================
    // ADDING ELEMENTS
    // ========================================================================

    /// Appends `element` to the back of the deque. O(1) amortized.
    ///
    /// Grows the ring buffer geometrically when full. The counterpart
    /// for the front end is `prepend`.
    ///
    /// # Examples
    ///
    /// ```
    /// var d = Deque[Int64]();
    /// d.append(1);
    /// d.append(2);
    /// d.popFirst();  // .Some(1)
    /// ```
    public mutating func append(element: T) {
        self.grow(minCapacity: self.len() + 1);
        let s = &mutating self.storage.value;
        var tail = s.head + s.len;
        if tail >= s.cap { tail = tail - s.cap; }
        unsafe s.ptr.offset(by: tail).write(element);
        s.len = s.len + 1
    }

    /// Prepends `element` to the front of the deque. O(1) amortized.
    ///
    /// Grows the ring buffer geometrically when full. The counterpart
    /// for the back end is `append`.
    ///
    /// # Examples
    ///
    /// ```
    /// var d = Deque[Int64]();
    /// d.prepend(1);
    /// d.prepend(0);
    /// d.popFirst();  // .Some(0)
    /// ```
    public mutating func prepend(element: T) {
        self.grow(minCapacity: self.len() + 1);
        let s = &mutating self.storage.value;
        s.head = s.head - 1;
        if s.head < 0 { s.head = s.cap - 1; }
        unsafe s.ptr.offset(by: s.head).write(element);
        s.len = s.len + 1
    }

    // ========================================================================
    // REMOVING ELEMENTS
    // ========================================================================

    /// Removes and returns the front element, or `.None` if empty. O(1).
    ///
    /// Advances the ring-buffer head by one slot. The non-removing mirror
    /// is `first`. The back-end counterpart is `popLast()`.
    ///
    /// # Examples
    ///
    /// ```
    /// var d = Deque[Int64](from: [1, 2, 3]);
    /// d.popFirst();  // .Some(1)
    /// d.popFirst();  // .Some(2)
    /// d.popFirst();  // .Some(3)
    /// d.popFirst();  // .None
    /// ```
    public mutating func popFirst() -> T? {
        if self.len() == 0 { return .None; }
        let s = &mutating self.storage.value;
        // Moved out: the slot leaves the live range, so nothing drops it.
        let v = unsafe s.ptr.offset(by: s.head).take();
        s.head = s.head + 1;
        if s.head >= s.cap { s.head = 0; }
        s.len = s.len - 1;
        .Some(v)
    }

    /// Removes and returns the back element, or `.None` if empty. O(1).
    ///
    /// Retracts the logical tail by one slot. The non-removing mirror
    /// is `last`. The front-end counterpart is `popFirst()`.
    ///
    /// # Examples
    ///
    /// ```
    /// var d = Deque[Int64](from: [1, 2, 3]);
    /// d.popLast();  // .Some(3)
    /// d.popLast();  // .Some(2)
    /// d.popLast();  // .Some(1)
    /// d.popLast();  // .None
    /// ```
    public mutating func popLast() -> T? {
        if self.len() == 0 { return .None; }
        let s = &mutating self.storage.value;
        s.len = s.len - 1;
        var tail = s.head + s.len;
        if tail >= s.cap { tail = tail - s.cap; }
        // Moved out: the slot leaves the live range, so nothing drops it.
        .Some(unsafe s.ptr.offset(by: tail).take())
    }

    // ========================================================================
    // ELEMENT ACCESS
    // ========================================================================

    /// Lends the front element without removing it, or `.None` if empty.
    ///
    /// O(1). The removing counterpart is `popFirst()`.
    ///
    /// # Examples
    ///
    /// ```
    /// let d = Deque[Int64](from: [10, 20, 30]);
    /// d.first;  // .Some(10)
    /// Deque[Int64]().first;  // .None
    /// ```
    public var first: Optional[&T] {
        let phys = self.physicalIndex(0);
        if phys < 0 { return .None; }
        .Some(unsafe self.rawPtr().offset(by: phys).value)
    }

    /// Lends the back element without removing it, or `.None` if empty.
    ///
    /// O(1). The removing counterpart is `popLast()`.
    ///
    /// # Examples
    ///
    /// ```
    /// let d = Deque[Int64](from: [10, 20, 30]);
    /// d.last;  // .Some(30)
    /// Deque[Int64]().last;  // .None
    /// ```
    public var last: Optional[&T] {
        let phys = self.physicalIndex(self.len() - 1);
        if phys < 0 { return .None; }
        .Some(unsafe self.rawPtr().offset(by: phys).value)
    }

    /// @name Indexed
    /// O(1) random access by logical index.
    ///
    /// Logical index 0 is the front element, `count - 1` is the back.
    /// The ring-buffer offset is computed internally. Both get and set
    /// are O(1).
    ///
    /// # Errors
    ///
    /// Panics with `"Deque: index out of bounds"` when `index < 0` or
    /// `index >= count`.
    ///
    /// # Examples
    ///
    /// ```
    /// var d = Deque[Int64](from: [10, 20, 30]);
    /// d(0);  // 10
    /// d(2);  // 30
    /// d(1) = 99;
    /// d(1);  // 99
    /// ```
    public subscript(index: Int64) -> T {
        ref {
            let phys = self.physicalIndex(index);
            if phys < 0 { fatalError("Deque: index out of bounds") }
            unsafe self.rawPtr().offset(by: phys).value
        }
        mutating ref {
            // Unique the storage (COW) before locating the slot: a copy
            // re-bases the ring to `head == 0`.
            self.storage.ensureUnique();
            let phys = self.physicalIndex(index);
            if phys < 0 { fatalError("Deque: index out of bounds") }
            unsafe self.rawPtr().offset(by: phys).mutatingValue
        }
    }

    /// Like `d(index)`, but the place may not exist: `.None` instead of a
    /// panic when out of range. `if let .Some(&mutating v) = d(checked: i)`
    /// writes in place.
    ///
    /// # Examples
    ///
    /// ```
    /// var d = Deque[Int64](from: [10, 20]);
    /// d(checked: 1);  // .Some(20)
    /// d(checked: 5);  // .None
    /// ```
    public subscript(checked index: Int64)? -> T {
        ref {
            let phys = self.physicalIndex(index);
            if phys < 0 { return null; }
            .Some(unsafe self.rawPtr().offset(by: phys).value)
        }
        mutating ref {
            self.storage.ensureUnique();
            let phys = self.physicalIndex(index);
            if phys < 0 { return null; }
            .Some(unsafe self.rawPtr().offset(by: phys).mutatingValue)
        }
    }

    /// The ring-buffer offset of logical `index`, or `-1` when out of range.
    fileprivate func physicalIndex(index: Int64) -> Int64 {
        if index < 0 or index >= self.len() { return -1; }
        let phys = self.head() + index;
        if phys >= self.cap() { return phys - self.cap(); }
        phys
    }

    /// Returns the two contiguous slices that make up the ring buffer.
    ///
    /// If the buffer doesn't wrap, the first slice contains all elements
    /// and the second is empty. If it wraps, the first slice covers
    /// head-to-end and the second covers start-to-tail. Both lend the
    /// deque's storage in place: nothing is copied, and they are only
    /// valid until the deque next changes.
    ///
    /// # Examples
    ///
    /// ```
    /// var d = Deque[Int64](from: [1, 2, 3]);
    /// let (a, b) = d.asSlices();
    /// // non-wrapping: a contains all 3 elements, b is empty
    /// ```
    public func asSlices() -> (&Slice[T], &Slice[T]) {
        let myLen = self.len();
        if myLen == 0 {
            return (
                unsafe Slice.lend(start: Pointer[T].nullPointer(), count: 0),
                unsafe Slice.lend(start: Pointer[T].nullPointer(), count: 0)
            )
        }
        let myHead = self.head();
        let myCap = self.cap();
        let myPtr = self.rawPtr();
        let tailEnd = myHead + myLen;
        if tailEnd <= myCap {
            return (
                unsafe Slice.lend(start: myPtr.offset(by: myHead), count: myLen),
                unsafe Slice.lend(start: myPtr, count: 0)
            )
        }
        let firstLen = myCap - myHead;
        return (
            unsafe Slice.lend(start: myPtr.offset(by: myHead), count: firstLen),
            unsafe Slice.lend(start: myPtr, count: myLen - firstLen)
        )
    }

    // ========================================================================
    // ITERATION
    // ========================================================================

    /// Returns an iterator that yields elements front-to-back.
    ///
    /// The iterator walks the ring buffer from `head` through `count`
    /// elements, wrapping around the buffer boundary transparently.
    ///
    /// # Examples
    ///
    /// ```
    /// var d = Deque[Int64](from: [10, 20, 30]);
    /// for x in d {
    ///     // yields 10, 20, 30
    /// }
    /// ```
    public func iter() -> DequeIterator[T] {
        DequeIterator(ptr: self.rawPtr(), cap: self.cap(), pos: self.head(), remaining: self.len())
    }

    // ========================================================================
    // CLONEABLE
    // ========================================================================

    /// Returns a shallow copy of this deque. O(1).
    ///
    /// The `CowBox` storage is shared until either copy mutates, at
    /// which point the COW barrier fires a deep clone that linearizes
    /// the ring buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// let d = Deque[Int64](from: [1, 2, 3]);
    /// var d2 = d.clone();
    /// d2.append(4);
    /// d.count;   // 3 -- original unchanged
    /// d2.count;  // 4
    /// ```
    public func clone() -> Deque[T, A] {
        var d = Deque[T, A](allocator: self.allocator);
        d.storage = self.storage.clone();
        d
    }
}

// Constructors that make their own allocator.
extend Deque[T, A] where A: Defaultable {
    /// @name Empty
    /// Creates an empty deque with no allocation.
    ///
    /// No heap memory is allocated until the first `append` or
    /// `prepend`. Use `Deque(capacity:)` to pre-allocate when the
    /// expected size is known.
    ///
    /// # Examples
    ///
    /// ```
    /// var d = Deque[Int64]();
    /// d.isEmpty;  // true
    /// ```
    public init() {
        self.init(allocator: A())
    }

    /// @name With Capacity
    /// Creates an empty deque with at least `capacity` slots reserved.
    ///
    /// Allocates a ring buffer that can hold `capacity` elements before
    /// needing to grow. Passing 0 is equivalent to `Deque()`.
    ///
    /// # Examples
    ///
    /// ```
    /// var d = Deque[Int64](capacity: 100);
    /// d.count;  // 0
    /// d.capacity;  // 100
    /// ```
    public init(capacity capacity: Int64) {
        self.init(capacity: capacity, allocator: A())
    }

    /// @name From Iterable
    /// Creates a deque by collecting every element from an iterable.
    ///
    /// Elements are appended via `append`, so iteration order of the
    /// source is preserved as front-to-back order in the deque.
    ///
    /// # Examples
    ///
    /// ```
    /// let d = Deque[Int64](from: 1..<5);
    /// d.count;  // 4
    /// d.first;  // .Some(1)
    /// d.last;   // .Some(4)
    /// ```
    public init[I](from iterable: I) where I: Iterable, I: not Static, I.Item = T {
        self.init();
        var iter = iterable.iter();
        while let .Some(item) = iter.next() {
            self.append(item)
        }
    }
}
