// Array type - dynamic growable array with COW semantics

module std.collections

import std.core.(Int, Bool, UInt8, UInt32, UInt64, Equatable, Comparable, Cloneable, Hashable, Hasher)
import std.result.(Optional)
import std.memory.(Allocator, ArcBox, Buffer, LiteralSlice)
import std.iter.(Iterator, Iterable, Collectable, Functor)
import std.ops.(ExpressibleByArrayLiteral)

public struct Array[T, A]:
    Iterable,
    Collectable,
    Functor,
    ExpressibleByArrayLiteral,
    Cloneable
    where A: Allocator
{
    type Iterable.Item = T
    type Collectable.Item = T
    type Functor.Inner = T
    type Iter = ArrayIterator[T, A]

    private var storage: ArcBox[ArrayStorage[T, A]]

    struct ArrayStorage[T1, A1] where A1: Allocator {
        var buffer: Buffer[T1, A1]
        var count: Int
    }

    // Constructors
    public init() {
        self.storage = ArcBox(value: ArrayStorage(
            buffer: Buffer(capacity: 0),
            count: 0
        ))
    }

    public init(allocator: A) {
        self.storage = ArcBox(value: ArrayStorage(
            buffer: Buffer(capacity: 0, allocator: allocator),
            count: 0
        ))
    }

    public init(capacity: Int) {
        self.storage = ArcBox(value: ArrayStorage(
            buffer: Buffer(capacity: capacity),
            count: 0
        ))
    }

    public init(capacity: Int, allocator: A) {
        self.storage = ArcBox(value: ArrayStorage(
            buffer: Buffer(capacity: capacity, allocator: allocator),
            count: 0
        ))
    }

    // ExpressibleByArrayLiteral
    public init(arrayLiteral elements: LiteralSlice[T]) {
        self.init(capacity: elements.count);
        for element in elements {
            self.append(element)
        }
    }

    // Collectable
    public init[I](from iter: I) where I: Iterator, I.Item = T {
        self.init();
        while let item = iter.next() {
            self.append(item)
        }
    }

    // Properties
    public var count: Int {
        self.storage.value.count
    }

    public var capacity: Int {
        self.storage.value.buffer.capacity
    }

    public var isEmpty: Bool {
        self.storage.value.count == 0
    }

    // COW helper
    private mutating func ensureUnique() {
        if not self.storage.isUnique() {
            self.storage = self.storage.deepClone()
        }
    }

    public mutating func ensureCapacity(capacity: Int) {
        self.ensureUnique();
        if self.storage.value.buffer.capacity < capacity {
            let newCapacity = if self.storage.value.buffer.capacity == 0 {
                if capacity < 4 { 4 } else { capacity }
            } else {
                var cap = self.storage.value.buffer.capacity;
                while cap < capacity {
                    cap = cap * 2
                }
                cap
            };
            self.storage.value.buffer.resize(to: newCapacity)
        }
    }

    //// Subscript variants
    //public subscript(safe index: Int) -> Optional[T] {
    //    get {
    //        if index >= 0 and index < self.count {
    //            .Some((self.storage.value.buffer)(unchecked: index))
    //        } else {
    //            .None
    //        }
    //    }
    //    set {
    //        if index >= 0 and index < self.count {
    //            if let value = newValue {
    //                self.ensureUnique()
    //                (self.storage.value.buffer)(unchecked: index) = value
    //            }
    //        }
    //    }
    //}

    //public subscript(wrapping index: Int) -> T {
    //    get {
    //        let n = self.count
    //        let wrapped = ((index % n) + n) % n
    //        (self.storage.value.buffer)(unchecked: wrapped)
    //    }
    //    set {
    //        let n = self.count
    //        let wrapped = ((index % n) + n) % n
    //        self.ensureUnique()
    //        (self.storage.value.buffer)(unchecked: wrapped) = newValue
    //    }
    //}

    public subscript(unchecked index: Int) -> T {
        get { (self.storage.value.buffer)(unchecked: index) }
        set {
            self.ensureUnique();
            (self.storage.value.buffer)(unchecked: index) = newValue
        }
    }

    //public subscript(safe range: Range[Int]) -> Optional[Slice[T]] {
    //    get {
    //        if range.start >= 0 and range.end <= self.count {
    //            self.storage.value.buffer.slice(from: range.start, to: range.end)
    //        } else {
    //            .None
    //        }
    //    }
    //}

    // Mutation
    public mutating func append(element: T) {
        self.ensureCapacity(self.count + 1);
        (self.storage.value.buffer)(unchecked: self.storage.value.count) = element;
        self.storage.value.count = self.storage.value.count + 1
    }

    public mutating func append(contentsOf other: Array[T, A]) {
        self.ensureCapacity(self.count + other.count);
        /* for i in 0..<other.count {
            (self.storage.value.buffer)(unchecked: self.storage.value.count) = other(unchecked: i)
            self.storage.value.count += 1
        } */
    }

    public mutating func insert(element: T, at index: Int) {
        if index < 0 or index > self.count {
            lang.panic("Array.insert: index out of bounds")
        }

        self.ensureCapacity(self.count + 1);

        // Shift elements right
        var i = self.storage.value.count;
        while i > index {
            (self.storage.value.buffer)(unchecked: i) = (self.storage.value.buffer)(unchecked: i - 1);
            i = i - 1
        }

        (self.storage.value.buffer)(unchecked: index) = element;
        self.storage.value.count = self.storage.value.count + 1
    }

    public mutating func remove(at index: Int) -> T {
        if index < 0 or index >= self.count {
            lang.panic("Array.remove: index out of bounds")
        }

        self.ensureUnique();
        let removed = (self.storage.value.buffer)(unchecked: index);

        // Shift elements left
        /* for i in index..<(self.storage.value.count - 1) {
            (self.storage.value.buffer)(unchecked: i) = (self.storage.value.buffer)(unchecked: i + 1)
        } */

        self.storage.value.count = self.storage.value.count - 1;
        removed
    }

    public mutating func pop() -> Optional[T] {
        if self.isEmpty {
            return .None
        }

        self.ensureUnique();
        self.storage.value.count = self.storage.value.count - 1;
        .Some((self.storage.value.buffer)(unchecked: self.storage.value.count))
    }

    public mutating func clear() {
        self.ensureUnique();
        self.storage.value.count = 0
    }

    public mutating func reserveCapacity(minimumCapacity: Int) {
        self.ensureCapacity(minimumCapacity)
    }

    // Access
    public func first() -> Optional[T] {
        if self.isEmpty {
            .None
        } else {
            .Some((self.storage.value.buffer)(unchecked: 0))
        }
    }

    public func last() -> Optional[T] {
        if self.isEmpty {
            .None
        } else {
            .Some((self.storage.value.buffer)(unchecked: self.count - 1))
        }
    }

    // Iteration
    public func iter() -> ArrayIterator[T, A] {
        ArrayIterator(array: self, index: 0)
    }

    // Functor
    public func map[U](transform: (T) -> U) -> Array[U, A] {
        var result = Array[U, A](capacity: self.count);
        /* for i in 0..<self.count {
            result.append(transform((self.storage.value.buffer)(unchecked: i)))
        } */
        result
    }

    // Cloneable
    public func clone() -> Array[T, A] where T: Cloneable {
        var result = Array[T, A](capacity: self.count);
        /* for i in 0..<self.count {
            result.append((self.storage.value.buffer)(unchecked: i).clone())
        } */
        result
    }

    // Sorting
    public mutating func sort() where T: Comparable {
        self.ensureUnique()
        // Simple insertion sort for now
        /* for i in 1..<self.count {
            let key = (self.storage.value.buffer)(unchecked: i)
            var j = i - 1
            while j >= 0 and (self.storage.value.buffer)(unchecked: j) > key {
                (self.storage.value.buffer)(unchecked: j + 1) = (self.storage.value.buffer)(unchecked: j)
                j -= 1
            }
            (self.storage.value.buffer)(unchecked: j + 1) = key
        } */
    }

    public func sorted() -> Array[T, A] where T: Comparable, T: Cloneable {
        var result = self.clone();
        result.sort();
        result
    }

    public mutating func reverse() {
        self.ensureUnique();
        var left = 0;
        var right = self.count - 1;
        while left < right {
            let temp = (self.storage.value.buffer)(unchecked: left);
            (self.storage.value.buffer)(unchecked: left) = (self.storage.value.buffer)(unchecked: right);
            (self.storage.value.buffer)(unchecked: right) = temp;
            left = left + 1;
            right = right - 1
        }
    }

    public func reversed() -> Array[T, A] where T: Cloneable {
        var result = self.clone();
        result.reverse();
        result
    }

    // Search
    public func contains(element: T) -> Bool where T: Equatable {
        /* for i in 0..<self.count {
            if (self.storage.value.buffer)(unchecked: i) == element {
                return true
            }
        } */
        false
    }

    public func indexOf(element: T) -> Optional[Int] where T: Equatable {
        /* for i in 0..<self.count {
            if (self.storage.value.buffer)(unchecked: i) == element {
                return .Some(i)
            }
        } */
        .None
    }
}

// Equatable when T is Equatable
extend Array[T, A]: Equatable where T: Equatable {
    public func equals(other: Array[T, A]) -> Bool {
        if self.count != other.count {
            return false
        }
        /* for i in 0..<self.count {
            if self(unchecked: i) != other(unchecked: i) {
                return false
            }
        } */
        true
    }
}

// Hashable when T is Hashable
extend Array[T, A]: Hashable where T: Hashable {
    public func hash[H](mutating into hasher: H) where H: Hasher {
        /* for i in 0..<self.count {
            self(unchecked: i).hash(into: hasher)
        } */
    }
}

// Array iterator
public struct ArrayIterator[T, A]: Iterator where A: Allocator {
    type Item = T

    private var array: Array[T, A]
    private var index: Int

    public init(array: Array[T, A], index: Int) {
        self.array = array;
        self.index = index;
    }

    public mutating func next() -> Optional[T] {
        if self.index < self.array.count {
            let value = self.array(unchecked: self.index);
            self.index = self.index + 1;
            .Some(value)
        } else {
            .None
        }
    }
}
