// Array[T] - dynamic growable array

module std.collections

import std.core.(Bool, Equatable, Comparable, Cloneable)
import std.num.(Int64)
import std.result.(Optional)
import std.memory.(Layout, Pointer, Slice, RawPointer, SystemAllocator, LiteralSlice)
import std.iter.(Iterator, Iterable)
import std.core.(ExpressibleByArrayLiteral, _ExpressibleByArrayLiteral)

// ArrayIterator must be defined before Array for Iterable conformance
public struct ArrayIterator[T]: Iterator {
    type Item = T

    private var ptr: Pointer[T]
    private var remaining: Int64

    public init(ptr: Pointer[T], count: Int64) {
        self.ptr = ptr;
        self.remaining = count;
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

// Array[T] - simple dynamic array using SystemAllocator
public struct Array[T]: Iterable, ExpressibleByArrayLiteral, _ExpressibleByArrayLiteral {
    type Item = T
    type Iter = ArrayIterator[T]
    type Element = T

    private var ptr: Pointer[T]
    private var len: Int64
    private var cap: Int64

    // Private init for internal use
    private init(ptr: Pointer[T], len: Int64, cap: Int64) {
        self.ptr = ptr;
        self.len = len;
        self.cap = cap;
    }

    // Create empty array
    public init() {
        self.ptr = Pointer(raw: lang.ptr_null[T]());
        self.len = Int64(intLiteral: 0);
        self.cap = Int64(intLiteral: 0);
    }

    // Create with capacity
    public init(capacity: Int64) {
        if capacity > Int64(intLiteral: 0) {
            let layout = Layout.array[T](capacity);
            var allocator = SystemAllocator();
            let result = allocator.allocate(layout);
            if result.isSome() {
                self.ptr = result.unwrap().cast[T]();
                self.len = Int64(intLiteral: 0);
                self.cap = capacity
            } else {
                lang.panic("Array allocation failed")
            }
        } else {
            self.ptr = Pointer(raw: lang.ptr_null[T]());
            self.len = Int64(intLiteral: 0);
            self.cap = Int64(intLiteral: 0)
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
                self.ptr = result.unwrap().cast[T]();
                self.len = Int64(intLiteral: 0);
                self.cap = elementCount;
                // Copy elements from literal slice
                var iter = elements.iter();
                var done: Bool = false;
                while done == false {
                    let item = iter.next();
                    if item.isSome() {
                        self.ptr.offset(by: self.len).write(item.unwrap());
                        self.len = self.len + Int64(intLiteral: 1)
                    } else {
                        done = true
                    }
                }
            } else {
                lang.panic("Array allocation failed")
            }
        } else {
            self.ptr = Pointer(raw: lang.ptr_null[T]());
            self.len = Int64(intLiteral: 0);
            self.cap = Int64(intLiteral: 0)
        }
    }

    deinit {
        if self.cap > Int64(intLiteral: 0) {
            let layout = Layout.array[T](self.cap);
            var allocator = SystemAllocator();
            allocator.deallocate(self.ptr.asRaw(), layout)
        }
    }

    // Properties
    public func count() -> Int64 { self.len }
    public func capacity() -> Int64 { self.cap }
    public func isEmpty() -> Bool { self.len == Int64(intLiteral: 0) }
    public func pointer() -> Pointer[T] { self.ptr }

    // Get a slice view of the array
    public func asSlice() -> Slice[T] {
        Slice(pointer: self.ptr, count: self.len)
    }

    // Grow capacity if needed
    private mutating func grow(minCapacity: Int64) {
        if self.cap >= minCapacity {
            return
        }

        // Calculate new capacity
        var newCap: Int64 = self.cap;
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
            // Copy existing elements
            var i: Int64 = Int64(intLiteral: 0);
            while i < self.len {
                newPtr.offset(by: i).write(self.ptr.offset(by: i).read());
                i = i + Int64(intLiteral: 1)
            }
            // Free old buffer
            if self.cap > Int64(intLiteral: 0) {
                let oldLayout = Layout.array[T](self.cap);
                allocator.deallocate(self.ptr.asRaw(), oldLayout)
            }
            self.ptr = newPtr;
            self.cap = newCap
        } else {
            lang.panic("Array grow failed")
        }
    }

    // Unchecked element access
    public func getUnchecked(index: Int64) -> T {
        self.ptr.offset(by: index).read()
    }

    // Unchecked write
    public func setUnchecked(index: Int64, value: T) {
        self.ptr.offset(by: index).write(value)
    }

    // Safe element access
    public func getValue(at index: Int64) -> Optional[T] {
        if index >= Int64(intLiteral: 0) and index < self.len {
            .Some(self.ptr.offset(by: index).read())
        } else {
            .None
        }
    }

    // Subscripts commented out due to compiler issue with parameter binding
    // public subscript(safe index: Int64) -> Optional[T] {
    //     get {
    //         if index >= Int64(intLiteral: 0) and index < self.len {
    //             .Some(self.ptr.offset(by: index).read())
    //         } else {
    //             .None
    //         }
    //     }
    // }
    // public subscript(unchecked index: Int64) -> T {
    //     get { self.ptr.offset(by: index).read() }
    //     set { self.ptr.offset(by: index).write(newValue) }
    // }

    // Append element
    public mutating func append(element: T) {
        self.grow(self.len + Int64(intLiteral: 1));
        self.ptr.offset(by: self.len).write(element);
        self.len = self.len + Int64(intLiteral: 1)
    }

    // Pop last element
    public mutating func pop() -> Optional[T] {
        if self.len > Int64(intLiteral: 0) {
            self.len = self.len - Int64(intLiteral: 1);
            .Some(self.ptr.offset(by: self.len).read())
        } else {
            .None
        }
    }

    // Clear all elements
    public mutating func clear() {
        self.len = Int64(intLiteral: 0)
    }

    // First element
    public func first() -> Optional[T] {
        if self.len > Int64(intLiteral: 0) {
            .Some(self.ptr.read())
        } else {
            .None
        }
    }

    // Last element
    public func last() -> Optional[T] {
        if self.len > Int64(intLiteral: 0) {
            .Some(self.ptr.offset(by: self.len - Int64(intLiteral: 1)).read())
        } else {
            .None
        }
    }

    // Insert at index
    public mutating func insert(element: T, at index: Int64) {
        if index < Int64(intLiteral: 0) or index > self.len {
            lang.panic("Array.insert: index out of bounds")
        }
        self.grow(self.len + Int64(intLiteral: 1));
        // Shift elements right
        var i: Int64 = self.len;
        while i > index {
            self.ptr.offset(by: i).write(self.ptr.offset(by: i - Int64(intLiteral: 1)).read());
            i = i - Int64(intLiteral: 1)
        }
        self.ptr.offset(by: index).write(element);
        self.len = self.len + Int64(intLiteral: 1)
    }

    // Remove at index
    public mutating func remove(at index: Int64) -> T {
        if index < Int64(intLiteral: 0) or index >= self.len {
            lang.panic("Array.remove: index out of bounds")
        }
        let removed = self.ptr.offset(by: index).read();
        // Shift elements left
        var i: Int64 = index;
        while i < self.len - Int64(intLiteral: 1) {
            self.ptr.offset(by: i).write(self.ptr.offset(by: i + Int64(intLiteral: 1)).read());
            i = i + Int64(intLiteral: 1)
        }
        self.len = self.len - Int64(intLiteral: 1);
        removed
    }

    // Iterable
    public func iter() -> ArrayIterator[T] {
        ArrayIterator(ptr: self.ptr, count: self.len)
    }

    // Reverse in place
    public mutating func reverse() {
        var left: Int64 = Int64(intLiteral: 0);
        var right: Int64 = self.len - Int64(intLiteral: 1);
        while left < right {
            let temp = self.ptr.offset(by: left).read();
            self.ptr.offset(by: left).write(self.ptr.offset(by: right).read());
            self.ptr.offset(by: right).write(temp);
            left = left + Int64(intLiteral: 1);
            right = right - Int64(intLiteral: 1)
        }
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

// Cloneable when T is Cloneable
extend Array[T]: Cloneable where T: Cloneable {
    public func clone() -> Array[T] {
        let selfCount = self.count();
        var result = Array(capacity: selfCount);
        var i: Int64 = Int64(intLiteral: 0);
        while i < selfCount {
            result.append(self.getUnchecked(i).clone());
            i = i + Int64(intLiteral: 1)
        }
        result
    }
}
