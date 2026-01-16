// Set[T] - hash set

module std.collections

import std.core.(Bool, Equatable, Cloneable)
import std.num.(Int64)
import std.result.(Optional)
import std.memory.(Layout, Pointer, RawPointer, SystemAllocator)
import std.iter.(Iterator, Iterable)

// Compute next power of two, minimum 8
func setNextPowerOfTwo(n: Int64) -> Int64 {
    var p: Int64 = Int64(intLiteral: 1);
    while p < n {
        p = p * Int64(intLiteral: 2)
    }
    let minCap = Int64(intLiteral: 8);
    if p < minCap { minCap } else { p }
}

// Entry in the set
public struct SetEntry[T] {
    public var element: T
    public var occupied: Bool

    public init(element: T, occupied: Bool) {
        self.element = element;
        self.occupied = occupied;
    }

    // Create an unoccupied entry with placeholder element
    public init(placeholder: T) {
        self.element = placeholder;
        self.occupied = false;
    }
}

// SetIterator must be defined before Set for Iterable conformance
public struct SetIterator[T]: Iterator {
    type Item = T

    private var entries: Pointer[SetEntry[T]]
    private var capacity: Int64
    private var index: Int64

    public init(entries: Pointer[SetEntry[T]], capacity: Int64) {
        self.entries = entries;
        self.capacity = capacity;
        self.index = Int64(intLiteral: 0);
    }

    public mutating func next() -> Optional[T] {
        while self.index < self.capacity {
            let entry = self.entries.offset(by: self.index).read();
            self.index = self.index + Int64(intLiteral: 1);
            if entry.occupied {
                return .Some(entry.element)
            }
        }
        let none: Optional[T] = .None;
        none
    }
}

// Set[T] - simple hash set using open addressing
// Note: Elements must be Equatable. Uses linear search (O(n)) since Hashable isn't complete.
public struct Set[T]: Iterable where T: Equatable {
    type Item = T
    type Iter = SetIterator[T]

    // Made internal so extensions can access
    var entries: Pointer[SetEntry[T]]
    var len: Int64
    var cap: Int64

    // Create empty set
    public init() {
        self.entries = Pointer(raw: lang.ptr_null[SetEntry[T]]());
        self.len = Int64(intLiteral: 0);
        self.cap = Int64(intLiteral: 0);
    }

    // Create with initial capacity - requires a placeholder element for initialization
    public init(capacity: Int64, placeholder: T) {
        let actualCap = setNextPowerOfTwo(capacity);
        if actualCap > Int64(intLiteral: 0) {
            let layout = Layout.array[SetEntry[T]](actualCap);
            var allocator = SystemAllocator();
            let result = allocator.allocate(layout);
            if result.isSome() {
                self.entries = result.unwrap().cast[SetEntry[T]]();
                self.len = Int64(intLiteral: 0);
                self.cap = actualCap;
                // Initialize all entries as unoccupied
                var i: Int64 = Int64(intLiteral: 0);
                while i < actualCap {
                    self.entries.offset(by: i).write(SetEntry(placeholder: placeholder));
                    i = i + Int64(intLiteral: 1)
                }
            } else {
                lang.panic("Set allocation failed")
            }
        } else {
            self.entries = Pointer(raw: lang.ptr_null[SetEntry[T]]());
            self.len = Int64(intLiteral: 0);
            self.cap = Int64(intLiteral: 0)
        }
    }

    deinit {
        if self.cap > Int64(intLiteral: 0) {
            let layout = Layout.array[SetEntry[T]](self.cap);
            var allocator = SystemAllocator();
            allocator.deallocate(self.entries.asRaw(), layout)
        }
    }

    // Properties
    public func count() -> Int64 { self.len }
    public func getCapacity() -> Int64 { self.cap }
    public func isEmpty() -> Bool { self.len == Int64(intLiteral: 0) }

    // Find entry by element using linear search
    private func findEntry(element: T) -> Optional[Int64] {
        if self.cap == Int64(intLiteral: 0) {
            let none: Optional[Int64] = .None;
            return none
        }

        var i: Int64 = Int64(intLiteral: 0);
        var result: Optional[Int64] = .None;
        var done: Bool = false;

        while i < self.cap and done == false {
            let entry = self.entries.offset(by: i).read();
            if entry.occupied and entry.element.equals(element) {
                result = .Some(i);
                done = true
            }
            i = i + Int64(intLiteral: 1)
        }
        result
    }

    // Find first unoccupied slot
    private func findEmptySlot() -> Optional[Int64] {
        var i: Int64 = Int64(intLiteral: 0);
        var result: Optional[Int64] = .None;
        var done: Bool = false;

        while i < self.cap and done == false {
            let entry = self.entries.offset(by: i).read();
            if entry.occupied == false {
                result = .Some(i);
                done = true
            }
            i = i + Int64(intLiteral: 1)
        }
        result
    }

    // Ensure we have capacity for more entries (resize at 75% load)
    private mutating func ensureCapacity(placeholder: T) {
        let threshold = self.cap * Int64(intLiteral: 3) / Int64(intLiteral: 4);
        if self.len >= threshold or self.cap == Int64(intLiteral: 0) {
            self.resize(placeholder)
        }
    }

    // Resize the set
    private mutating func resize(placeholder: T) {
        let newCap: Int64 = if self.cap == Int64(intLiteral: 0) {
            Int64(intLiteral: 8)
        } else {
            self.cap * Int64(intLiteral: 2)
        };

        let oldEntries = self.entries;
        let oldCap = self.cap;

        // Allocate new table
        let layout = Layout.array[SetEntry[T]](newCap);
        var allocator = SystemAllocator();
        let result = allocator.allocate(layout);
        if result.isSome() {
            self.entries = result.unwrap().cast[SetEntry[T]]();
            self.cap = newCap;
            self.len = Int64(intLiteral: 0);

            // Initialize new entries
            var i: Int64 = Int64(intLiteral: 0);
            while i < newCap {
                self.entries.offset(by: i).write(SetEntry(placeholder: placeholder));
                i = i + Int64(intLiteral: 1)
            }

            // Copy old entries
            i = Int64(intLiteral: 0);
            while i < oldCap {
                let entry = oldEntries.offset(by: i).read();
                if entry.occupied {
                    // Find empty slot and insert
                    let maybeSlot = self.findEmptySlot();
                    if maybeSlot.isSome() {
                        self.entries.offset(by: maybeSlot.unwrap()).write(entry);
                        self.len = self.len + Int64(intLiteral: 1)
                    }
                }
                i = i + Int64(intLiteral: 1)
            }

            // Free old table
            if oldCap > Int64(intLiteral: 0) {
                let oldLayout = Layout.array[SetEntry[T]](oldCap);
                allocator.deallocate(oldEntries.asRaw(), oldLayout)
            }
        } else {
            lang.panic("Set resize failed")
        }
    }

    // Check if element exists
    public func contains(element: T) -> Bool {
        self.findEntry(element).isSome()
    }

    // Insert element, returns true if element was new
    public mutating func insert(element: T) -> Bool {
        // Check if already exists
        if self.findEntry(element).isSome() {
            return false
        }

        // Need to insert - ensure capacity first
        self.ensureCapacity(element);

        // Find empty slot
        let maybeSlot = self.findEmptySlot();
        if maybeSlot.isSome() {
            self.entries.offset(by: maybeSlot.unwrap()).write(SetEntry(
                element: element,
                occupied: true
            ));
            self.len = self.len + Int64(intLiteral: 1)
        } else {
            lang.panic("Set insert failed - no empty slot")
        }
        true
    }

    // Remove element, returns true if element was present
    public mutating func remove(element: T) -> Bool {
        let maybeIndex = self.findEntry(element);

        if maybeIndex.isSome() {
            let index = maybeIndex.unwrap();
            let entry = self.entries.offset(by: index).read();

            // Mark as unoccupied (keep element as placeholder)
            self.entries.offset(by: index).write(SetEntry(placeholder: entry.element));
            self.len = self.len - Int64(intLiteral: 1);
            true
        } else {
            false
        }
    }

    // Clear all entries
    public mutating func clear() {
        var i: Int64 = Int64(intLiteral: 0);
        while i < self.cap {
            let entry = self.entries.offset(by: i).read();
            // Keep element but mark unoccupied
            self.entries.offset(by: i).write(SetEntry(placeholder: entry.element));
            i = i + Int64(intLiteral: 1)
        }
        self.len = Int64(intLiteral: 0)
    }

    // Iteration
    public func iter() -> SetIterator[T] {
        SetIterator(entries: self.entries, capacity: self.cap)
    }

    // Get internal data for extensions
    public func getEntries() -> Pointer[SetEntry[T]] { self.entries }

    // Set operations

    // Union: elements in either set
    public func union(other: Set[T]) -> Set[T] {
        let selfCount = self.count();
        let otherCount = other.count();

        if selfCount == Int64(intLiteral: 0) and otherCount == Int64(intLiteral: 0) {
            return Set()
        }

        // Find a placeholder element
        var placeholder: T = self.entries.offset(by: Int64(intLiteral: 0)).read().element;
        if selfCount > Int64(intLiteral: 0) {
            var i: Int64 = Int64(intLiteral: 0);
            var found: Bool = false;
            while i < self.cap and found == false {
                let entry = self.entries.offset(by: i).read();
                if entry.occupied {
                    placeholder = entry.element;
                    found = true
                }
                i = i + Int64(intLiteral: 1)
            }
        } else {
            var i: Int64 = Int64(intLiteral: 0);
            var found: Bool = false;
            while i < other.cap and found == false {
                let entry = other.entries.offset(by: i).read();
                if entry.occupied {
                    placeholder = entry.element;
                    found = true
                }
                i = i + Int64(intLiteral: 1)
            }
        }

        var result = Set(capacity: selfCount + otherCount, placeholder: placeholder);

        // Add all from self
        var i: Int64 = Int64(intLiteral: 0);
        while i < self.cap {
            let entry = self.entries.offset(by: i).read();
            if entry.occupied {
                let _ = result.insert(entry.element);
            }
            i = i + Int64(intLiteral: 1)
        }

        // Add all from other
        i = Int64(intLiteral: 0);
        while i < other.cap {
            let entry = other.entries.offset(by: i).read();
            if entry.occupied {
                let _ = result.insert(entry.element);
            }
            i = i + Int64(intLiteral: 1)
        }

        result
    }

    // Intersection: elements in both sets
    public func intersection(other: Set[T]) -> Set[T] {
        let selfCount = self.count();

        if selfCount == Int64(intLiteral: 0) {
            return Set()
        }

        // Find a placeholder element
        var placeholder: T = self.entries.offset(by: Int64(intLiteral: 0)).read().element;
        var i: Int64 = Int64(intLiteral: 0);
        var found: Bool = false;
        while i < self.cap and found == false {
            let entry = self.entries.offset(by: i).read();
            if entry.occupied {
                placeholder = entry.element;
                found = true
            }
            i = i + Int64(intLiteral: 1)
        }

        var result = Set(capacity: selfCount, placeholder: placeholder);

        // Add elements that are in both
        i = Int64(intLiteral: 0);
        while i < self.cap {
            let entry = self.entries.offset(by: i).read();
            if entry.occupied and other.contains(entry.element) {
                let _ = result.insert(entry.element);
            }
            i = i + Int64(intLiteral: 1)
        }

        result
    }

    // Difference: elements in self but not in other
    public func difference(other: Set[T]) -> Set[T] {
        let selfCount = self.count();

        if selfCount == Int64(intLiteral: 0) {
            return Set()
        }

        // Find a placeholder element
        var placeholder: T = self.entries.offset(by: Int64(intLiteral: 0)).read().element;
        var i: Int64 = Int64(intLiteral: 0);
        var found: Bool = false;
        while i < self.cap and found == false {
            let entry = self.entries.offset(by: i).read();
            if entry.occupied {
                placeholder = entry.element;
                found = true
            }
            i = i + Int64(intLiteral: 1)
        }

        var result = Set(capacity: selfCount, placeholder: placeholder);

        // Add elements not in other
        i = Int64(intLiteral: 0);
        while i < self.cap {
            let entry = self.entries.offset(by: i).read();
            if entry.occupied and other.contains(entry.element) == false {
                let _ = result.insert(entry.element);
            }
            i = i + Int64(intLiteral: 1)
        }

        result
    }

    // Symmetric difference: elements in either but not both
    public func symmetricDifference(other: Set[T]) -> Set[T] {
        let selfCount = self.count();
        let otherCount = other.count();

        if selfCount == Int64(intLiteral: 0) and otherCount == Int64(intLiteral: 0) {
            return Set()
        }

        // Find a placeholder element
        var placeholder: T = self.entries.offset(by: Int64(intLiteral: 0)).read().element;
        if selfCount > Int64(intLiteral: 0) {
            var i: Int64 = Int64(intLiteral: 0);
            var found: Bool = false;
            while i < self.cap and found == false {
                let entry = self.entries.offset(by: i).read();
                if entry.occupied {
                    placeholder = entry.element;
                    found = true
                }
                i = i + Int64(intLiteral: 1)
            }
        } else {
            var i: Int64 = Int64(intLiteral: 0);
            var found: Bool = false;
            while i < other.cap and found == false {
                let entry = other.entries.offset(by: i).read();
                if entry.occupied {
                    placeholder = entry.element;
                    found = true
                }
                i = i + Int64(intLiteral: 1)
            }
        }

        var result = Set(capacity: selfCount + otherCount, placeholder: placeholder);

        // Add elements in self but not other
        var i: Int64 = Int64(intLiteral: 0);
        while i < self.cap {
            let entry = self.entries.offset(by: i).read();
            if entry.occupied and other.contains(entry.element) == false {
                let _ = result.insert(entry.element);
            }
            i = i + Int64(intLiteral: 1)
        }

        // Add elements in other but not self
        i = Int64(intLiteral: 0);
        while i < other.cap {
            let entry = other.entries.offset(by: i).read();
            if entry.occupied and self.contains(entry.element) == false {
                let _ = result.insert(entry.element);
            }
            i = i + Int64(intLiteral: 1)
        }

        result
    }

    // Subset check: all elements in self are in other
    public func isSubset(other: Set[T]) -> Bool {
        var allFound: Bool = true;
        var i: Int64 = Int64(intLiteral: 0);
        while i < self.cap and allFound {
            let entry = self.entries.offset(by: i).read();
            if entry.occupied and other.contains(entry.element) == false {
                allFound = false
            }
            i = i + Int64(intLiteral: 1)
        }
        allFound
    }

    // Superset check: all elements in other are in self
    public func isSuperset(other: Set[T]) -> Bool {
        other.isSubset(self)
    }

    // Disjoint check: no common elements
    public func isDisjoint(other: Set[T]) -> Bool {
        var noCommon: Bool = true;
        var i: Int64 = Int64(intLiteral: 0);
        while i < self.cap and noCommon {
            let entry = self.entries.offset(by: i).read();
            if entry.occupied and other.contains(entry.element) {
                noCommon = false
            }
            i = i + Int64(intLiteral: 1)
        }
        noCommon
    }
}

// Equatable extension
extend Set[T]: Equatable where T: Equatable {
    public func equals(other: Set[T]) -> Bool {
        let selfCount = self.count();
        let otherCount = other.count();
        if selfCount != otherCount {
            return false
        }

        // Check all elements in self exist in other
        var equal: Bool = true;
        var i: Int64 = Int64(intLiteral: 0);
        let selfCap = self.getCapacity();
        let selfEntries = self.getEntries();
        while i < selfCap and equal {
            let entry = selfEntries.offset(by: i).read();
            if entry.occupied and other.contains(entry.element) == false {
                equal = false
            }
            i = i + Int64(intLiteral: 1)
        }
        equal
    }
}

// Cloneable extension
extend Set[T]: Cloneable where T: Cloneable {
    public func clone() -> Set[T] {
        let selfCount = self.count();
        let selfCap = self.getCapacity();
        let selfEntries = self.getEntries();

        if selfCount == Int64(intLiteral: 0) {
            return Set()
        }

        // Find first element to use as placeholder
        var placeholder: T = selfEntries.offset(by: Int64(intLiteral: 0)).read().element;
        var i: Int64 = Int64(intLiteral: 0);
        var found: Bool = false;
        while i < selfCap and found == false {
            let entry = selfEntries.offset(by: i).read();
            if entry.occupied {
                placeholder = entry.element;
                found = true
            }
            i = i + Int64(intLiteral: 1)
        }

        var result = Set(capacity: selfCount, placeholder: placeholder.clone());
        i = Int64(intLiteral: 0);
        while i < selfCap {
            let entry = selfEntries.offset(by: i).read();
            if entry.occupied {
                let _ = result.insert(entry.element.clone());
            }
            i = i + Int64(intLiteral: 1)
        }
        result
    }
}
