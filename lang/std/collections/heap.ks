// Heap[T] - binary min-heap backed by Array

module std.collections

import std.core.(Bool, Cloneable, Comparable, fatalError)
import std.numeric.(Int64)
import std.result.(Optional)
import std.memory.(ArraySliceIterator)
import std.iter.(Iterator, Iterable)
import std.collections.(Array)

/// Binary min-heap backed by `Array[T]`.
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
/// A single `Array[T]` field in standard binary-heap layout: the minimum
/// lives at index 0, children of node `i` are at `2i + 1` and `2i + 2`.
///
/// # Memory Model
///
/// Delegates storage to `Array[T]`, inheriting its COW value semantics.
/// Copying a `Heap` is O(1); the first mutation on a shared copy triggers
/// the array's copy-on-write barrier.
///
/// # Guarantees
///
/// - `peek()` always returns the minimum element.
/// - After `pop()`, the next-smallest element becomes the new minimum.
/// - Iteration order is unspecified (internal heap layout).
public struct Heap[T]: Iterable, Cloneable where T: Comparable {
    /// `Iterable` element type.
    type Item = T
    /// `Iterable` iterator type — reuses the backing array's iterator.
    type TargetIterator = ArraySliceIterator[T]

    private var data: Array[T]

    // ========================================================================
    // CONSTRUCTORS
    // ========================================================================

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
        self.data = Array[T]();
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
        self.data = Array[T](capacity: capacity);
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
    public init[I](from iterable: I) where I: Iterable, I.Item = T {
        self.data = Array[T]();
        var iter = iterable.iter();
        while let .Some(item) = iter.next() {
            self.data.append(item)
        }
        self.heapify();
    }

    /// @name From Data
    /// Internal constructor that wraps an existing array as the heap's
    /// backing store. Used by `clone()` to avoid a redundant heapify.
    init(data data: Array[T]) {
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
        let min = self.data(0);
        self.data.swap(at: 0, with: n - 1);
         self.data.pop();
        self.siftDown(0);
        .Some(min)
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
    /// If the current capacity already meets or exceeds `capacity`, this
    /// is a no-op. Otherwise the backing array grows to accommodate the
    /// requested number of slots.
    ///
    /// # Examples
    ///
    /// ```
    /// var h = Heap[Int64]();
    /// h.reserveCapacity(minimumCapacity: 100);
    /// ```
    public mutating func reserveCapacity(minimumCapacity capacity: Int64) {
        self.data.reserveCapacity(capacity)
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
    public func iter() -> ArraySliceIterator[T] {
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
    public func clone() -> Heap[T] {
        Heap(data: self.data.clone())
    }

    // ========================================================================
    // INTERNAL
    // ========================================================================

    /// Bubbles element at `index` upward until the min-heap invariant holds.
    private mutating func siftUp(index: Int64) {
        var i = index;
        while i > 0 {
            let parent = (i - 1) / 2;
            if self.data(i) < self.data(parent) {
                self.data.swap(at: i, with: parent);
                i = parent
            } else {
                break
            }
        }
    }

    /// Pushes element at `index` downward until the min-heap invariant holds.
    private mutating func siftDown(index: Int64) {
        let n = self.data.count;
        var i = index;
        loop {
            let left = 2 * i + 1;
            let right = 2 * i + 2;
            var smallest = i;
            if left < n and self.data(left) < self.data(smallest) {
                smallest = left
            }
            if right < n and self.data(right) < self.data(smallest) {
                smallest = right
            }
            if smallest == i { break; }
            self.data.swap(at: i, with: smallest);
            i = smallest
        }
    }

    /// Floyd's algorithm: sift down from the last internal node to the root.
    private mutating func heapify() {
        var i = self.data.count / 2 - 1;
        while i >= 0 {
            self.siftDown(i);
            i = i - 1
        }
    }
}
