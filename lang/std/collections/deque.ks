// Deque[T] - ring-buffer double-ended queue with COW semantics

module std.collections

import std.core.(Bool, Cloneable, fatalError)
import std.numeric.(Int64)
import std.result.(Optional)
import std.memory.(Layout, Pointer, ArraySlice, SystemAllocator, CowBox)
import std.iter.(Iterator, Iterable)

// ============================================================================
// DEQUE STORAGE (Internal)
// ============================================================================

/// Ring-buffer storage cell for `Deque[T]`.
///
/// Wrapped in a `CowBox` by `Deque[T]` so that copying a deque simply
/// bumps a reference count; mutations call `CowBox.write()` first to
/// perform the copy-on-write barrier. The `clone()` method linearizes
/// the ring buffer into a fresh contiguous allocation.
struct DequeStorage[T]: Cloneable {
    /// Heap pointer to the element buffer; null when `cap == 0`.
    var ptr: Pointer[T]
    /// Number of initialized elements in the ring buffer.
    var len: Int64
    /// Total slots allocated; always `>= len`.
    var cap: Int64
    /// Index of the logical first element in the ring buffer.
    var head: Int64

    /// @name From Fields
    /// Constructs a `DequeStorage` from raw fields.
    ///
    /// The caller must guarantee `len <= cap`, `head < cap` (or both 0),
    /// and `ptr` valid for `cap` elements when `cap > 0`.
    init(ptr ptr: Pointer[T], len len: Int64, cap cap: Int64, head head: Int64) {
        self.ptr = ptr;
        self.len = len;
        self.cap = cap;
        self.head = head;
    }

    /// Deep-copies the storage, linearizing the ring buffer into a fresh
    /// contiguous allocation sized exactly to `len`.
    ///
    /// The new buffer starts with `head == 0`, so elements are laid out
    /// contiguously regardless of where they wrapped in the source.
    /// Returns an empty storage with a null pointer when `len == 0`.
    /// Panics if allocation fails.
    func clone() -> DequeStorage[T] {
        if self.len == 0 {
            return DequeStorage(
                ptr: Pointer[T].nullPointer(),
                len: 0,
                cap: 0,
                head: 0
            )
        }
        let layout = Layout.array[T](self.len);
        var allocator = SystemAllocator();
        let result = allocator.allocate(layout);
        if let .Some(rawPtr) = result {
            let newPtr = rawPtr.cast[T]();
            for i in 0..<self.len {
                var phys = self.head + i;
                if phys >= self.cap { phys = phys - self.cap; }
                newPtr.offset(by: i).write(self.ptr.offset(by: phys).read());
            }
            DequeStorage(ptr: newPtr, len: self.len, cap: self.len, head: 0)
        } else {
            fatalError("DequeStorage clone allocation failed")
        }
    }

    /// Frees the underlying ring buffer if one was allocated.
    deinit {
        if self.cap > 0 {
            let layout = Layout.array[T](self.cap);
            var allocator = SystemAllocator();
            allocator.deallocate(self.ptr.asRaw(), layout)
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
        let value = self.ptr.offset(by: self.pos).read();
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
/// O(1) amortized `pushBack`/`pushFront`/`popBack`/`popFront` and O(1)
/// random access by index. Storage is shared between copies until one
/// mutates, at which point the COW barrier fires.
///
/// # Examples
///
/// ```
/// var d = Deque[Int64]();
/// d.pushBack(1);
/// d.pushFront(0);
/// d.pushBack(2);
/// d.popFront();  // .Some(0)
/// d.popBack();   // .Some(2)
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
/// - `pushBack`/`pushFront` are O(1) amortized; growth is geometric.
/// - `popBack`/`popFront` are O(1).
/// - Subscript access is O(1).
/// - Iteration order is front-to-back.
public struct Deque[T]: Iterable, Cloneable {
    /// `Iterable` element type.
    type Item = T
    /// `Iterable` iterator type.
    type TargetIterator = DequeIterator[T]

    /// COW-wrapped ring-buffer storage.
    fileprivate var storage: CowBox[DequeStorage[T]]

    // -- internal helpers --

    /// Reads the current element count from the COW storage.
    fileprivate func len() -> Int64 { self.storage.read().len }
    /// Reads the current buffer capacity from the COW storage.
    fileprivate func cap() -> Int64 { self.storage.read().cap }
    /// Reads the ring-buffer head offset from the COW storage.
    fileprivate func head() -> Int64 { self.storage.read().head }
    /// Reads the raw element pointer from the COW storage.
    fileprivate func rawPtr() -> Pointer[T] { self.storage.read().ptr }

    // ========================================================================
    // CONSTRUCTORS
    // ========================================================================

    /// @name Empty
    /// Creates an empty deque with no allocation.
    ///
    /// No heap memory is allocated until the first `pushBack` or
    /// `pushFront`. Use `Deque(capacity:)` to pre-allocate when the
    /// expected size is known.
    ///
    /// # Examples
    ///
    /// ```
    /// var d = Deque[Int64]();
    /// d.isEmpty;  // true
    /// ```
    public init() {
        self.storage = CowBox(DequeStorage(
            ptr: Pointer[T].nullPointer(),
            len: 0,
            cap: 0,
            head: 0
        ));
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
        if capacity > 0 {
            let layout = Layout.array[T](capacity);
            var allocator = SystemAllocator();
            let result = allocator.allocate(layout);
            if let .Some(rawPtr) = result {
                self.storage = CowBox(DequeStorage(
                    ptr: rawPtr.cast[T](),
                    len: 0,
                    cap: capacity,
                    head: 0
                ))
            } else {
                fatalError("Deque allocation failed")
            }
        } else {
            self.storage = CowBox(DequeStorage(
                ptr: Pointer[T].nullPointer(),
                len: 0,
                cap: 0,
                head: 0
            ))
        }
    }

    /// @name From Iterable
    /// Creates a deque by collecting every element from an iterable.
    ///
    /// Elements are appended via `pushBack`, so iteration order of the
    /// source is preserved as front-to-back order in the deque.
    ///
    /// # Examples
    ///
    /// ```
    /// let d = Deque[Int64](from: 1..<5);
    /// d.count;  // 4
    /// d.first();  // .Some(1)
    /// d.last();   // .Some(4)
    /// ```
    public init[I](from iterable: I) where I: Iterable, I.Item = T {
        self.init();
        var iter = iterable.iter();
        while let .Some(item) = iter.next() {
            self.pushBack(item)
        }
    }

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
        var allocator = SystemAllocator();
        let result = allocator.allocate(newLayout);
        if let .Some(rawPtr) = result {
            let newPtr = rawPtr.cast[T]();
            // write() fires the COW barrier before we read the old storage
            var oldStorage = self.storage.write();
            // Linearize ring buffer into new contiguous buffer
            for i in 0..<oldStorage.len {
                var phys = oldStorage.head + i;
                if phys >= oldStorage.cap { phys = phys - oldStorage.cap; }
                newPtr.offset(by: i).write(oldStorage.ptr.offset(by: phys).read());
            }
            let oldLen = oldStorage.len;
            self.storage.setValue(DequeStorage(
                ptr: newPtr, len: oldLen, cap: newCap, head: 0
            ))
        } else {
            fatalError("Deque grow failed")
        }
    }

    /// Ensures the buffer can hold at least `capacity` elements without
    /// reallocating.
    ///
    /// If the current capacity already meets or exceeds `capacity`, this
    /// is a no-op. Otherwise the ring buffer is reallocated and
    /// linearized. Useful before a burst of `pushBack`/`pushFront` calls
    /// when the final size is known in advance.
    ///
    /// # Examples
    ///
    /// ```
    /// var d = Deque[Int64]();
    /// d.reserveCapacity(minimumCapacity: 100);
    /// ```
    public mutating func reserveCapacity(minimumCapacity capacity: Int64) {
        if capacity > self.cap() {
            self.grow(minCapacity: capacity)
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
        var s = self.storage.write();
        s.len = 0;
        s.head = 0;
        self.storage.setValue(s)
    }

    // ========================================================================
    // ADDING ELEMENTS
    // ========================================================================

    /// Appends `element` to the back of the deque. O(1) amortized.
    ///
    /// Grows the ring buffer geometrically when full. The counterpart
    /// for the front end is `pushFront`.
    ///
    /// # Examples
    ///
    /// ```
    /// var d = Deque[Int64]();
    /// d.pushBack(1);
    /// d.pushBack(2);
    /// d.popFront();  // .Some(1)
    /// ```
    public mutating func pushBack(element: T) {
        self.grow(minCapacity: self.len() + 1);
        var s = self.storage.write();
        var tail = s.head + s.len;
        if tail >= s.cap { tail = tail - s.cap; }
        s.ptr.offset(by: tail).write(element);
        s.len = s.len + 1;
        self.storage.setValue(s)
    }

    /// Prepends `element` to the front of the deque. O(1) amortized.
    ///
    /// Grows the ring buffer geometrically when full. The counterpart
    /// for the back end is `pushBack`.
    ///
    /// # Examples
    ///
    /// ```
    /// var d = Deque[Int64]();
    /// d.pushFront(1);
    /// d.pushFront(0);
    /// d.popFront();  // .Some(0)
    /// ```
    public mutating func pushFront(element: T) {
        self.grow(minCapacity: self.len() + 1);
        var s = self.storage.write();
        s.head = s.head - 1;
        if s.head < 0 { s.head = s.cap - 1; }
        s.ptr.offset(by: s.head).write(element);
        s.len = s.len + 1;
        self.storage.setValue(s)
    }

    // ========================================================================
    // REMOVING ELEMENTS
    // ========================================================================

    /// Removes and returns the front element, or `.None` if empty. O(1).
    ///
    /// Advances the ring-buffer head by one slot. The non-removing mirror
    /// is `first()`. The back-end counterpart is `popBack()`.
    ///
    /// # Examples
    ///
    /// ```
    /// var d = Deque[Int64](from: [1, 2, 3]);
    /// d.popFront();  // .Some(1)
    /// d.popFront();  // .Some(2)
    /// d.popFront();  // .Some(3)
    /// d.popFront();  // .None
    /// ```
    public mutating func popFront() -> T? {
        if self.len() == 0 { return .None; }
        var s = self.storage.write();
        let value = s.ptr.offset(by: s.head).read();
        s.head = s.head + 1;
        if s.head >= s.cap { s.head = 0; }
        s.len = s.len - 1;
        self.storage.setValue(s);
        .Some(value)
    }

    /// Removes and returns the back element, or `.None` if empty. O(1).
    ///
    /// Retracts the logical tail by one slot. The non-removing mirror
    /// is `last()`. The front-end counterpart is `popFront()`.
    ///
    /// # Examples
    ///
    /// ```
    /// var d = Deque[Int64](from: [1, 2, 3]);
    /// d.popBack();  // .Some(3)
    /// d.popBack();  // .Some(2)
    /// d.popBack();  // .Some(1)
    /// d.popBack();  // .None
    /// ```
    public mutating func popBack() -> T? {
        if self.len() == 0 { return .None; }
        var s = self.storage.write();
        s.len = s.len - 1;
        var tail = s.head + s.len;
        if tail >= s.cap { tail = tail - s.cap; }
        let value = s.ptr.offset(by: tail).read();
        self.storage.setValue(s);
        .Some(value)
    }

    // ========================================================================
    // ELEMENT ACCESS
    // ========================================================================

    /// Returns the front element without removing it, or `.None` if empty.
    ///
    /// O(1). The removing counterpart is `popFront()`.
    ///
    /// # Examples
    ///
    /// ```
    /// let d = Deque[Int64](from: [10, 20, 30]);
    /// d.first();  // .Some(10)
    /// Deque[Int64]().first();  // .None
    /// ```
    public func first() -> T? {
        if self.len() == 0 { return .None; }
        .Some(self.rawPtr().offset(by: self.head()).read())
    }

    /// Returns the back element without removing it, or `.None` if empty.
    ///
    /// O(1). The removing counterpart is `popBack()`.
    ///
    /// # Examples
    ///
    /// ```
    /// let d = Deque[Int64](from: [10, 20, 30]);
    /// d.last();  // .Some(30)
    /// Deque[Int64]().last();  // .None
    /// ```
    public func last() -> T? {
        if self.len() == 0 { return .None; }
        var tail = self.head() + self.len() - 1;
        if tail >= self.cap() { tail = tail - self.cap(); }
        .Some(self.rawPtr().offset(by: tail).read())
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
    subscript(index: Int64) -> T {
        get {
            if index < 0 or index >= self.len() {
                fatalError("Deque: index out of bounds")
            }
            var phys = self.head() + index;
            let myCap = self.cap();
            if phys >= myCap { phys = phys - myCap; }
            self.rawPtr().offset(by: phys).read()
        }
        set {
            if index < 0 or index >= self.len() {
                fatalError("Deque: index out of bounds")
            }
            var s = self.storage.write();
            var phys = s.head + index;
            if phys >= s.cap { phys = phys - s.cap; }
            s.ptr.offset(by: phys).write(newValue);
            self.storage.setValue(s)
        }
    }

    /// Returns the two contiguous slices that make up the ring buffer.
    ///
    /// If the buffer doesn't wrap, the first slice contains all elements
    /// and the second is empty. If it wraps, the first slice covers
    /// head-to-end and the second covers start-to-tail. Useful for
    /// bulk operations that need pointer-contiguous access without
    /// copying.
    ///
    /// # Examples
    ///
    /// ```
    /// var d = Deque[Int64](from: [1, 2, 3]);
    /// let (a, b) = d.asSlices();
    /// // non-wrapping: a contains all 3 elements, b is empty
    /// ```
    public func asSlices() -> (ArraySlice[T], ArraySlice[T]) {
        let myLen = self.len();
        if myLen == 0 {
            return (
                ArraySlice(pointer: Pointer[T].nullPointer(), count: 0),
                ArraySlice(pointer: Pointer[T].nullPointer(), count: 0)
            )
        }
        let myHead = self.head();
        let myCap = self.cap();
        let myPtr = self.rawPtr();
        let tailEnd = myHead + myLen;
        if tailEnd <= myCap {
            return (
                ArraySlice(pointer: myPtr.offset(by: myHead), count: myLen),
                ArraySlice(pointer: myPtr, count: 0)
            )
        }
        let firstLen = myCap - myHead;
        return (
            ArraySlice(pointer: myPtr.offset(by: myHead), count: firstLen),
            ArraySlice(pointer: myPtr, count: myLen - firstLen)
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
    /// d2.pushBack(4);
    /// d.count;   // 3 -- original unchanged
    /// d2.count;  // 4
    /// ```
    public func clone() -> Deque[T] {
        var d = Deque[T]();
        d.storage = self.storage.clone();
        d
    }
}
