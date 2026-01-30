// Array[T] - dynamic growable array with COW (Copy-on-Write) semantics

module std.collections

import std.core.(Bool, Equatable, Comparable, Cloneable)
import std.num.(Int64)
import std.result.(Optional)
import std.memory.(Layout, Pointer, Slice, RawPointer, SystemAllocator, LiteralSlice, RcBox)
import std.iter.(Iterator, Iterable)
import std.core.(ExpressibleByArrayLiteral, _ExpressibleByArrayLiteral)

// ============================================================================
// ARRAY ITERATOR
// ============================================================================

/// Iterator over array elements.
public struct ArrayIterator[T]: Iterator {
    type Item = T

    private var ptr: Pointer[T]
    private var remaining: Int64

    /// Creates an array iterator.
    public init(ptr ptr: Pointer[T], remaining remaining: Int64) {
        self.ptr = ptr;
        self.remaining = remaining;
    }

    /// Returns the next element, or None if exhausted.
    public mutating func next() -> Optional[T] {
        if self.remaining > Int64(intLiteral: 0) {
            let value = self.ptr.read();
            self.ptr = self.ptr.offset(by: Int64(intLiteral: 1));
            self.remaining = self.remaining - Int64(intLiteral: 1);
            .Some(value)
        } else {
            .None
        }
    }
}

// ============================================================================
// ARRAY STORAGE (Internal)
// ============================================================================

/// Internal storage for Array (ptr, len, cap).
struct ArrayStorage[T]: Cloneable {
    var ptr: Pointer[T]
    var len: Int64
    var cap: Int64

    init(ptr ptr: Pointer[T], len len: Int64, cap cap: Int64) {
        self.ptr = ptr;
        self.len = len;
        self.cap = cap;
    }

    /// Deep clone - allocate new buffer and copy elements.
    func clone() -> ArrayStorage[T] {
        if self.len == Int64(intLiteral: 0) {
            return ArrayStorage(
                ptr: Pointer(raw: lang.ptr_null[T]()),
                len: Int64(intLiteral: 0),
                cap: Int64(intLiteral: 0)
            )
        }
        let layout = Layout.array[T](self.len);
        var allocator = SystemAllocator();
        let result = allocator.allocate(layout);
        if result.isSome() {
            let newPtr = result.unwrap().cast[T]();
            // Copy elements
            var i: Int64 = Int64(intLiteral: 0);
            while i < self.len {
                newPtr.offset(by: i).write(self.ptr.offset(by: i).read());
                i = i + Int64(intLiteral: 1)
            }
            ArrayStorage(ptr: newPtr, len: self.len, cap: self.len)
        } else {
            lang.panic("ArrayStorage clone allocation failed")
        }
    }

    deinit {
        if self.cap > Int64(intLiteral: 0) {
            let layout = Layout.array[T](self.cap);
            var allocator = SystemAllocator();
            allocator.deallocate(self.ptr.asRaw(), layout)
        }
    }
}

// ============================================================================
// ARRAY
// ============================================================================

/// A dynamic growable array with copy-on-write semantics.
///
/// Arrays automatically grow when elements are added and use COW
/// to efficiently share data between copies until mutation occurs.
@builtin(.ArrayStruct)
public struct Array[T]: Iterable, ExpressibleByArrayLiteral, _ExpressibleByArrayLiteral, Cloneable {
    type Item = T
    type Iter = ArrayIterator[T]
    type Element = T

    private var storage: RcBox[ArrayStorage[T]]

    // Helper accessors for storage fields
    private func ptr() -> Pointer[T] { self.storage.getValue().ptr }
    private func len() -> Int64 { self.storage.getValue().len }
    private func cap() -> Int64 { self.storage.getValue().cap }

    // Ensure unique storage for mutation (COW)
    private mutating func makeUnique() {
        if self.storage.isUnique() == false {
            self.storage = self.storage.deepClone()
        }
    }

    /// Private init for internal use (from storage).
    private init(storage storage: RcBox[ArrayStorage[T]]) {
        self.storage = storage;
    }

    // ========================================================================
    // CONSTRUCTORS
    // ========================================================================

    /// Creates an empty array.
    public init() {
        self.storage = RcBox(ArrayStorage(
            ptr: Pointer(raw: lang.ptr_null[T]()),
            len: Int64(intLiteral: 0),
            cap: Int64(intLiteral: 0)
        ));
    }

    /// Creates an empty array with the specified capacity.
    public init(capacity capacity: Int64) {
        if capacity > Int64(intLiteral: 0) {
            let layout = Layout.array[T](capacity);
            var allocator = SystemAllocator();
            let result = allocator.allocate(layout);
            if result.isSome() {
                self.storage = RcBox(ArrayStorage(
                    ptr: result.unwrap().cast[T](),
                    len: Int64(intLiteral: 0),
                    cap: capacity
                ))
            } else {
                lang.panic("Array allocation failed")
            }
        } else {
            self.storage = RcBox(ArrayStorage(
                ptr: Pointer(raw: lang.ptr_null[T]()),
                len: Int64(intLiteral: 0),
                cap: Int64(intLiteral: 0)
            ))
        }
    }

    /// Internal initializer called by compiler for array literals.
    public init(_arrayLiteralPointer: lang.ptr[T], _arrayLiteralCount: lang.i64) {
        self.init(arrayLiteral: LiteralSlice(pointer: _arrayLiteralPointer, count: _arrayLiteralCount))
    }

    /// Creates an array from an array literal.
    public init(arrayLiteral elements: LiteralSlice[T]) {
        let elementCount = elements.count();
        if elementCount > Int64(intLiteral: 0) {
            let layout = Layout.array[T](elementCount);
            var allocator = SystemAllocator();
            let result = allocator.allocate(layout);
            if result.isSome() {
                let newPtr = result.unwrap().cast[T]();
                var currentLen: Int64 = Int64(intLiteral: 0);
                // Copy elements from literal slice
                var iter = elements.iter();
                var done: Bool = false;
                while done == false {
                    let item = iter.next();
                    if item.isSome() {
                        newPtr.offset(by: currentLen).write(item.unwrap());
                        currentLen = currentLen + Int64(intLiteral: 1)
                    } else {
                        done = true
                    }
                }
                self.storage = RcBox(ArrayStorage(
                    ptr: newPtr,
                    len: currentLen,
                    cap: elementCount
                ))
            } else {
                lang.panic("Array allocation failed")
            }
        } else {
            self.storage = RcBox(ArrayStorage(
                ptr: Pointer(raw: lang.ptr_null[T]()),
                len: Int64(intLiteral: 0),
                cap: Int64(intLiteral: 0)
            ))
        }
    }

    // ========================================================================
    // SIZE & CAPACITY
    // ========================================================================

    /// The number of elements in the array.
    public func count() -> Int64 { self.len() }

    /// The allocated capacity.
    public func capacity() -> Int64 { self.cap() }

    /// True if the array is empty.
    public func isEmpty() -> Bool { self.len() == Int64(intLiteral: 0) }

    /// Returns a pointer to the underlying storage.
    public func pointer() -> Pointer[T] { self.ptr() }

    /// Returns a slice view of the array.
    public func asSlice() -> Slice[T] {
        Slice(pointer: self.ptr(), count: self.len())
    }

    // ========================================================================
    // CAPACITY MANAGEMENT (Internal)
    // ========================================================================

    /// Grows capacity to at least minCapacity.
    private mutating func grow(minCapacity: Int64) {
        let myCap = self.cap();
        if myCap >= minCapacity {
            return
        }

        self.makeUnique();

        // Calculate new capacity
        var newCap: Int64 = myCap;
        if newCap == Int64(intLiteral: 0) {
            newCap = Int64(intLiteral: 4)
        }
        while newCap < minCapacity {
            newCap = newCap * Int64(intLiteral: 2)
        }

        // Allocate new buffer
        let newLayout = Layout.array[T](newCap);
        var allocator = SystemAllocator();
        let result = allocator.allocate(newLayout);
        if result.isSome() {
            let newPtr = result.unwrap().cast[T]();
            let oldStorage = self.storage.getValue();
            // Copy existing elements
            var i: Int64 = Int64(intLiteral: 0);
            while i < oldStorage.len {
                newPtr.offset(by: i).write(oldStorage.ptr.offset(by: i).read());
                i = i + Int64(intLiteral: 1)
            }
            // Free old buffer
            if oldStorage.cap > Int64(intLiteral: 0) {
                let oldLayout = Layout.array[T](oldStorage.cap);
                allocator.deallocate(oldStorage.ptr.asRaw(), oldLayout)
            }
            self.storage.setValue(ArrayStorage(ptr: newPtr, len: oldStorage.len, cap: newCap))
        } else {
            lang.panic("Array grow failed")
        }
    }

    // ========================================================================
    // ELEMENT ACCESS
    // ========================================================================

    /// Returns the element at the given index without bounds checking.
    public func getUnchecked(index: Int64) -> T {
        self.ptr().offset(by: index).read()
    }

    /// Sets the element at the given index without bounds checking.
    public mutating func setUnchecked(index: Int64, value: T) {
        self.makeUnique();
        self.ptr().offset(by: index).write(value)
    }

    /// Returns the element at the given index, or None if out of bounds.
    public func getValue(at index: Int64) -> Optional[T] {
        let myLen = self.len();
        if index >= Int64(intLiteral: 0) and index < myLen {
            .Some(self.ptr().offset(by: index).read())
        } else {
            .None
        }
    }

    // ========================================================================
    // MUTATING OPERATIONS
    // ========================================================================

    /// Appends an element to the end of the array.
    public mutating func append(element: T) {
        let myLen = self.len();
        self.grow(myLen + Int64(intLiteral: 1));
        self.makeUnique();
        var s = self.storage.getValue();
        s.ptr.offset(by: s.len).write(element);
        s.len = s.len + Int64(intLiteral: 1);
        self.storage.setValue(s)
    }

    /// Removes and returns the last element, or None if empty.
    public mutating func pop() -> Optional[T] {
        let myLen = self.len();
        if myLen > Int64(intLiteral: 0) {
            self.makeUnique();
            var s = self.storage.getValue();
            s.len = s.len - Int64(intLiteral: 1);
            let value = s.ptr.offset(by: s.len).read();
            self.storage.setValue(s);
            .Some(value)
        } else {
            .None
        }
    }

    /// Removes all elements from the array.
    public mutating func clear() {
        self.makeUnique();
        var s = self.storage.getValue();
        s.len = Int64(intLiteral: 0);
        self.storage.setValue(s)
    }

    /// Returns the first element, or None if empty.
    public func first() -> Optional[T] {
        if self.len() > Int64(intLiteral: 0) {
            .Some(self.ptr().read())
        } else {
            .None
        }
    }

    /// Returns the last element, or None if empty.
    public func last() -> Optional[T] {
        let myLen = self.len();
        if myLen > Int64(intLiteral: 0) {
            .Some(self.ptr().offset(by: myLen - Int64(intLiteral: 1)).read())
        } else {
            .None
        }
    }

    /// Inserts an element at the specified index.
    ///
    /// Panics if index is out of bounds.
    public mutating func insert(element: T, at index: Int64) {
        let myLen = self.len();
        if index < Int64(intLiteral: 0) or index > myLen {
            lang.panic("Array.insert: index out of bounds")
        }
        self.grow(myLen + Int64(intLiteral: 1));
        self.makeUnique();
        var s = self.storage.getValue();
        // Shift elements right
        var i: Int64 = s.len;
        while i > index {
            s.ptr.offset(by: i).write(s.ptr.offset(by: i - Int64(intLiteral: 1)).read());
            i = i - Int64(intLiteral: 1)
        }
        s.ptr.offset(by: index).write(element);
        s.len = s.len + Int64(intLiteral: 1);
        self.storage.setValue(s)
    }

    /// Removes and returns the element at the specified index.
    ///
    /// Panics if index is out of bounds.
    public mutating func remove(at index: Int64) -> T {
        let myLen = self.len();
        if index < Int64(intLiteral: 0) or index >= myLen {
            lang.panic("Array.remove: index out of bounds")
        }
        self.makeUnique();
        var s = self.storage.getValue();
        let removed = s.ptr.offset(by: index).read();
        // Shift elements left
        var i: Int64 = index;
        while i < s.len - Int64(intLiteral: 1) {
            s.ptr.offset(by: i).write(s.ptr.offset(by: i + Int64(intLiteral: 1)).read());
            i = i + Int64(intLiteral: 1)
        }
        s.len = s.len - Int64(intLiteral: 1);
        self.storage.setValue(s);
        removed
    }

    // ========================================================================
    // ITERATION
    // ========================================================================

    /// Returns an iterator over the elements.
    public func iter() -> ArrayIterator[T] {
        ArrayIterator(ptr: self.ptr(), remaining: self.len())
    }

    // ========================================================================
    // TRANSFORMATIONS
    // ========================================================================

    /// Reverses the array in place.
    public mutating func reverse() {
        self.makeUnique();
        var s = self.storage.getValue();
        var left: Int64 = Int64(intLiteral: 0);
        var right: Int64 = s.len - Int64(intLiteral: 1);
        while left < right {
            let temp = s.ptr.offset(by: left).read();
            s.ptr.offset(by: left).write(s.ptr.offset(by: right).read());
            s.ptr.offset(by: right).write(temp);
            left = left + Int64(intLiteral: 1);
            right = right - Int64(intLiteral: 1)
        }
        self.storage.setValue(s)
    }

    // ========================================================================
    // PROTOCOL CONFORMANCES
    // ========================================================================

    /// Creates a shallow clone (COW - copy deferred until mutation).
    public func clone() -> Array[T] {
        Array(storage: self.storage.clone())
    }
}

// ============================================================================
// CONDITIONAL EXTENSIONS
// ============================================================================

/// Equatable extension when T is Equatable.
extend Array[T]: Equatable where T: Equatable {
    /// Compares two arrays for equality.
    public func equals(other: Array[T]) -> Bool {
        let selfCount = self.count();
        let otherCount = other.count();
        if selfCount != otherCount {
            return false
        }
        var i: Int64 = Int64(intLiteral: 0);
        var equal: Bool = true;
        while i < selfCount and equal {
            if self.getUnchecked(i).equals(other.getUnchecked(i)) == false {
                equal = false
            }
            i = i + Int64(intLiteral: 1)
        }
        equal
    }
}


// ============================================================================
// TYPE OPERATOR
// ============================================================================

/// Type operator alias: [T] desugars to Array[T].
@builtin(.ArrayTypeOperator)
public type ArrayTypeOperator[T] = Array[T];
