// Heap[T, A] - binary min-heap backed by Array

module std.collections

import std.core.(Bool, Cloneable, Comparable, Ordering, fatalError)
import std.numeric.(Int64)
import std.result.(Optional)

import std.iter.(Iterator, Iterable)
import std.collections.(Array, ArrayIterator)
import std.memory.(Allocator, GlobalAllocator)
import std.core.(Defaultable)

/// Binary min-heap backed by `Array[T, A]`.
///
/// O(log n) `push`/`pop`, O(1) `peek` at the minimum element. Builds
/// from an existing array in O(n) via Floyd's heapify. Iteration yields
/// elements in storage order (NOT sorted order).
///
/// # Examples
///
/// ```
/// var h = Heap[Int64]();
/// h.push(5);
/// h.push(1);
/// h.push(3);
/// h.peek();   // .Some(1)
/// h.pop();    // .Some(1)
/// h.pop();    // .Some(3)
/// ```
///
/// # Representation
///
/// A single `Array[T, A]` field in standard binary-heap layout: the minimum
/// lives at index 0, children of node `i` are at `2i + 1` and `2i + 2`.
///
/// # Memory Model
///
/// Delegates storage to `Array[T, A]`, inheriting its COW value semantics.
/// Copying a `Heap` is O(1); the first mutation on a shared copy triggers
/// the array's copy-on-write barrier.
///
/// # Guarantees
///
/// - `peek()` always returns the minimum element.
/// - After `pop()`, the next-smallest element becomes the new minimum.
/// - Iteration order is unspecified (internal heap layout).
public struct Heap[T, A = GlobalAllocator]: Iterable, Cloneable where T: Comparable, A: Allocator {
    /// `Iterable` element type.
    type Item = T
    /// `Iterable` iterator type — reuses the backing array's iterator.
    type TargetIterator = ArrayIterator[T, A]

    fileprivate var data: Array[T, A]

    // ========================================================================
    // CONSTRUCTORS
    // ========================================================================

    /// @name With Allocator
    /// An empty heap whose storage will come from `allocator`.
    public init(allocator allocator: A) {
        self.data = Array[T, A](allocator: allocator);
    }

    /// @name With Capacity And Allocator
    /// An empty heap with room for `capacity` elements, allocated from
    /// `allocator`.
    public init(capacity capacity: Int64, allocator allocator: A) {
        self.data = Array[T, A](capacity: capacity, allocator: allocator);
    }

    /// The allocator this heap's storage comes from.
    public var allocator: A { self.data.allocator }

    /// @name From Data
    /// Internal constructor that wraps an existing array as the heap's
    /// backing store. Used by `clone()` to avoid a redundant heapify.
    init(data data: Array[T, A]) {
        self.data = data;
    }

    // ========================================================================
    // PROPERTIES
    // ========================================================================

    /// Number of elements in the heap.
    ///
    /// # Examples
    ///
    /// ```
    /// let h = Heap(from: [3, 1, 2]);
    /// h.count;  // 3
    /// ```
    public var count: Int64 { self.data.count }

    /// True when the heap contains no elements.
    ///
    /// # Examples
    ///
    /// ```
    /// Heap[Int64]().isEmpty;  // true
    /// Heap(from: [1]).isEmpty;  // false
    /// ```
    public var isEmpty: Bool { self.data.isEmpty }

    // ========================================================================
    // CORE OPERATIONS
    // ========================================================================

    /// Inserts `element`, maintaining the min-heap invariant. O(log n).
    ///
    /// The element is appended to the backing array then sifted up to
    /// restore the heap property. Amortized O(log n) because the array
    /// may occasionally grow its buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// var h = Heap[Int64]();
    /// h.push(3);
    /// h.push(1);
    /// h.peek();  // .Some(1)
    /// ```
    public mutating func push(element: T) {
        self.data.append(element);
        self.siftUp(self.data.count - 1);
    }

    /// Removes and returns the minimum element, or `.None` if empty. O(log n).
    ///
    /// Swaps the root with the last element, removes the last, then sifts
    /// the new root down to restore the heap invariant. The non-removing
    /// mirror is `peek()`.
    ///
    /// # Examples
    ///
    /// ```
    /// var h = Heap(from: [3, 1, 2]);
    /// h.pop();  // .Some(1)
    /// h.pop();  // .Some(2)
    /// h.pop();  // .Some(3)
    /// h.pop();  // .None
    /// ```
    public mutating func pop() -> T? {
        if self.data.isEmpty { return .None; }
        let n = self.data.count;
        if n == 1 {
            return self.data.pop()
        }
        // Swap the minimum to the end and pop it: a move, not a copy.
        self.data.swap(at: 0, with: n - 1);
        let min = self.data.pop();
        self.siftDown(0);
        min
    }

    /// Returns the minimum element without removing it. O(1).
    ///
    /// Returns `.None` on an empty heap. The removing counterpart is
    /// `pop()`.
    ///
    /// # Examples
    ///
    /// ```
    /// let h = Heap(from: [3, 1, 2]);
    /// h.peek();  // .Some(1)
    /// Heap[Int64]().peek();  // .None
    /// ```
    public func peek() -> T? {
        if self.data.isEmpty { return .None; }
        .Some(self.data(0))
    }

    /// Removes all elements, retaining allocated capacity.
    ///
    /// After calling `clear()`, `count` is 0 but the backing array keeps
    /// its buffer so subsequent `push` calls avoid reallocation.
    ///
    /// # Examples
    ///
    /// ```
    /// var h = Heap(from: [3, 1, 2]);
    /// h.clear();
    /// h.isEmpty;  // true
    /// ```
    public mutating func clear() {
        self.data.clear()
    }

    /// Ensures the backing array can hold at least `capacity` elements
    /// without reallocating.
    ///
    /// If the current capacity already meets or exceeds `minimumCapacity`, this
    /// is a no-op. Otherwise the backing array grows to accommodate the
    /// requested number of slots.
    ///
    /// # Examples
    ///
    /// ```
    /// var h = Heap[Int64]();
    /// h.reserveCapacity(100);
    /// ```
    public mutating func reserveCapacity(minimumCapacity: Int64) {
        self.data.reserveCapacity(minimumCapacity)
    }

    // ========================================================================
    // ITERATION
    // ========================================================================

    /// Returns an iterator over elements in storage order (NOT sorted).
    ///
    /// The iteration order reflects the internal heap layout, not the
    /// sorted order. To consume elements smallest-first, use `pop()` in
    /// a loop instead.
    ///
    /// # Examples
    ///
    /// ```
    /// let h = Heap(from: [5, 3, 1]);
    /// var iter = h.iter();
    /// // yields elements in heap-array order, not 1, 3, 5
    /// ```
    public func iter() -> ArrayIterator[T, A] {
        self.data.iter()
    }

    // ========================================================================
    // CLONEABLE
    // ========================================================================

    /// Returns a shallow copy of this heap. O(1).
    ///
    /// The backing array's copy-on-write semantics mean the actual deep
    /// copy is deferred until the first mutation on either the original
    /// or the clone.
    ///
    /// # Examples
    ///
    /// ```
    /// let h = Heap(from: [3, 1, 2]);
    /// var h2 = h.clone();
    /// h2.push(0);
    /// h.peek();   // .Some(1) -- original unchanged
    /// h2.peek();  // .Some(0)
    /// ```
    public func clone() -> Heap[T, A] {
        Heap(data: self.data.clone())
    }

    // ========================================================================
    // INTERNAL
    // ========================================================================

    // The sifts move a hole instead of swapping: the moving element is
    // taken out once, each step moves one neighbour into the hole, and the
    // element goes into the final hole. Elements are reached through the
    // buffer pointer, past the bounds and copy-on-write checks (the buffer
    // is made unique first).

    /// Moves the element at `index` up until its parent is no greater.
    private mutating func siftUp(index: Int64) {
        let p = self.data.asMutableSlice().asUnsafeSlice().pointer;
        let moving = unsafe p.offset(by: index).take();
        var hole = index;
        while hole > 0 {
            let parent = (hole - 1) / 2;
            if moving.compare(unsafe p.offset(by: parent).value) != Ordering.Less { break }
            unsafe p.offset(by: hole).write(unsafe p.offset(by: parent).take());
            hole = parent
        }
        unsafe p.offset(by: hole).write(moving)
    }

    /// Moves the element at `index` down until no child is smaller.
    private mutating func siftDown(index: Int64) {
        let n = self.data.count;
        let p = self.data.asMutableSlice().asUnsafeSlice().pointer;
        let moving = unsafe p.offset(by: index).take();
        var hole = index;
        loop {
            var child = 2 * hole + 1;
            if child >= n { break }
            let right = child + 1;
            if right < n and unsafe p.offset(by: right).value.compare(unsafe p.offset(by: child).value) == Ordering.Less {
                child = right
            }
            if unsafe p.offset(by: child).value.compare(moving) != Ordering.Less { break }
            unsafe p.offset(by: hole).write(unsafe p.offset(by: child).take());
            hole = child
        }
        unsafe p.offset(by: hole).write(moving)
    }

    fileprivate mutating func heapify() {
        var i = self.data.count / 2 - 1;
        while i >= 0 {
            self.siftDown(i);
            i = i - 1
        }
    }
}

// Constructors that make their own allocator.
extend Heap[T, A] where A: Defaultable {
    /// @name Empty
    /// Creates an empty min-heap with no allocation.
    ///
    /// No heap memory is allocated until the first `push`. Use
    /// `Heap(capacity:)` to pre-allocate when the expected size is known.
    ///
    /// # Examples
    ///
    /// ```
    /// var h = Heap[Int64]();
    /// h.isEmpty;  // true
    /// ```
    public init() {
        self.init(allocator: A());
    }

    /// @name With Capacity
    /// Creates an empty min-heap with at least `capacity` slots reserved.
    ///
    /// Pre-allocates the backing array so that up to `capacity` elements
    /// can be pushed without triggering a reallocation.
    ///
    /// # Examples
    ///
    /// ```
    /// var h = Heap[Int64](capacity: 100);
    /// h.count;  // 0
    /// ```
    public init(capacity capacity: Int64) {
        self.init(capacity: capacity, allocator: A());
    }

    /// @name From Iterable
    /// Builds a min-heap by collecting elements then heapifying in O(n).
    ///
    /// All elements are first appended to the backing array, then
    /// Floyd's algorithm establishes the heap invariant in a single
    /// bottom-up pass. This is faster than pushing elements one at a
    /// time (O(n) vs O(n log n)).
    ///
    /// # Examples
    ///
    /// ```
    /// let h = Heap(from: [5, 3, 1, 4, 2]);
    /// h.peek();   // .Some(1)
    /// h.count;    // 5
    /// ```
    public init[I](from iterable: I) where I: Iterable, I: not Static, I.Item = T {
        self.init(allocator: A());
        var iter = iterable.iter();
        while let .Some(item) = iter.next() {
            self.data.append(item)
        }
        self.heapify();
    }
}
