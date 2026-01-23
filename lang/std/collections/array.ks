// Array[T] - dynamic growable array with COW (Copy-on-Write) semantics

module std.collections

import std.core.(Bool, Equatable, Comparable, Cloneable)
import std.num.(Int64)
import std.result.(Optional)
import std.memory.(Layout, Pointer, Slice, RawPointer, SystemAllocator, LiteralSlice, RcBox)
import std.iter.(Iterator, Iterable)
import std.core.(ExpressibleByArrayLiteral, _ExpressibleByArrayLiteral)

// ArrayIterator must be defined before Array for Iterable conformance
public struct ArrayIterator[T]: Iterator {
    type Item = T

    private var ptr: Pointer[T]
    private var remaining: Int64

    public init(ptr ptr: Pointer[T], remaining remaining: Int64) {
        self.ptr = ptr;
        self.remaining = remaining;
    }

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

// ArrayStorage[T] - internal storage for Array (ptr, len, cap)
struct ArrayStorage[T]: Cloneable {
    var ptr: Pointer[T]
    var len: Int64
    var cap: Int64

    init(ptr ptr: Pointer[T], len len: Int64, cap cap: Int64) {
        self.ptr = ptr;
        self.len = len;
        self.cap = cap;
    }

    // Deep clone - allocate new buffer and copy elements
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

// Array[T] - dynamic array with COW semantics using RcBox
@builtin(.ArrayStruct)
public struct Array[T]: Iterable, ExpressibleByArrayLiteral, _ExpressibleByArrayLiteral {
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

    // Private init for internal use (from storage)
    private init(storage storage: RcBox[ArrayStorage[T]]) {
        self.storage = storage;
    }

    // Create empty array
    public init() {
        self.storage = RcBox(ArrayStorage(
            ptr: Pointer(raw: lang.ptr_null[T]()),
            len: Int64(intLiteral: 0),
            cap: Int64(intLiteral: 0)
        ));
    }

    // Create with capacity
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

    // _ExpressibleByArrayLiteral (called by compiler for array literals)
    public init(_arrayLiteralPointer: lang.ptr[T], _arrayLiteralCount: lang.i64) {
        self.init(arrayLiteral: LiteralSlice(pointer: _arrayLiteralPointer, count: _arrayLiteralCount))
    }

    // ExpressibleByArrayLiteral
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

    // Properties
    public func count() -> Int64 { self.len() }
    public func capacity() -> Int64 { self.cap() }
    public func isEmpty() -> Bool { self.len() == Int64(intLiteral: 0) }
    public func pointer() -> Pointer[T] { self.ptr() }

    // Get a slice view of the array
    public func asSlice() -> Slice[T] {
        Slice(pointer: self.ptr(), count: self.len())
    }

    // Grow capacity if needed
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

    // Unchecked element access
    public func getUnchecked(index: Int64) -> T {
        self.ptr().offset(by: index).read()
    }

    // Unchecked write
    public mutating func setUnchecked(index: Int64, value: T) {
        self.makeUnique();
        self.ptr().offset(by: index).write(value)
    }

    // Safe element access
    public func getValue(at index: Int64) -> Optional[T] {
        let myLen = self.len();
        if index >= Int64(intLiteral: 0) and index < myLen {
            .Some(self.ptr().offset(by: index).read())
        } else {
            .None
        }
    }

    // Append element
    public mutating func append(element: T) {
        let myLen = self.len();
        self.grow(myLen + Int64(intLiteral: 1));
        self.makeUnique();
        var s = self.storage.getValue();
        s.ptr.offset(by: s.len).write(element);
        s.len = s.len + Int64(intLiteral: 1);
        self.storage.setValue(s)
    }

    // Pop last element
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

    // Clear all elements
    public mutating func clear() {
        self.makeUnique();
        var s = self.storage.getValue();
        s.len = Int64(intLiteral: 0);
        self.storage.setValue(s)
    }

    // First element
    public func first() -> Optional[T] {
        if self.len() > Int64(intLiteral: 0) {
            .Some(self.ptr().read())
        } else {
            .None
        }
    }

    // Last element
    public func last() -> Optional[T] {
        let myLen = self.len();
        if myLen > Int64(intLiteral: 0) {
            .Some(self.ptr().offset(by: myLen - Int64(intLiteral: 1)).read())
        } else {
            .None
        }
    }

    // Insert at index
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

    // Remove at index
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

    // Iterable
    public func iter() -> ArrayIterator[T] {
        ArrayIterator(ptr: self.ptr(), remaining: self.len())
    }

    // Reverse in place
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

    // Cloneable - shallow clone (COW)
    public func clone() -> Array[T] {
        Array(storage: self.storage.clone())
    }
}

// Equatable when T is Equatable
extend Array[T]: Equatable where T: Equatable {
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

// Cloneable conformance
extend Array[T]: Cloneable {}

// Type operator alias: [T] desugars to ArrayTypeOperator[T] which is Array[T]
@builtin(.ArrayTypeOperator)
public type ArrayTypeOperator[T] = Array[T];
