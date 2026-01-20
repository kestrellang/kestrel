// Iterator adapter types

module std.iter

import std.result.(Optional)
import std.core.(Bool, Cloneable)
import std.num.(Int64)
import std.iter.(Iterator)

// MapIterator - transforms each element
public struct MapIterator[I, U]: Iterator where I: Iterator {
    type Item = U

    private var inner: I
    private var transform: (I.Item) -> U

    public init(inner: I, transform: (I.Item) -> U) {
        self.inner = inner;
        self.transform = transform;
    }

    public mutating func next() -> Optional[U] {
        let item = self.inner.next();
        if item.isSome() {
            .Some(self.transform(item.unwrap()))
        } else {
            .None
        }
    }
}

// FilterIterator - yields only elements matching predicate
public struct FilterIterator[I]: Iterator where I: Iterator {
    type Item = I.Item

    private var inner: I
    private var predicate: (I.Item) -> Bool

    public init(inner: I, predicate: (I.Item) -> Bool) {
        self.inner = inner;
        self.predicate = predicate;
    }

    public mutating func next() -> Optional[I.Item] {
        var done: Bool = false;
        var result: Optional[I.Item] = .None;
        while done == false {
            let item = self.inner.next();
            if item.isNone() {
                done = true
            } else {
                let value = item.unwrap();
                if self.predicate(value) {
                    result = .Some(value);
                    done = true
                }
            }
        }
        result
    }
}

// FilterMapIterator - filters and transforms in one step
public struct FilterMapIterator[I, U]: Iterator where I: Iterator {
    type Item = U

    private var inner: I
    private var transform: (I.Item) -> Optional[U]

    public init(inner: I, transform: (I.Item) -> Optional[U]) {
        self.inner = inner;
        self.transform = transform;
    }

    public mutating func next() -> Optional[U] {
        var done: Bool = false;
        var result: Optional[U] = .None;
        while done == false {
            let item = self.inner.next();
            if item.isNone() {
                done = true
            } else {
                let transformed = self.transform(item.unwrap());
                if transformed.isSome() {
                    result = transformed;
                    done = true
                }
            }
        }
        result
    }
}

// TakeWhileIterator - takes elements while predicate is true
public struct TakeWhileIterator[I]: Iterator where I: Iterator {
    type Item = I.Item

    private var inner: I
    private var predicate: (I.Item) -> Bool
    private var done: Bool

    public init(inner: I, predicate: (I.Item) -> Bool) {
        self.inner = inner;
        self.predicate = predicate;
        self.done = false;
    }

    public mutating func next() -> Optional[I.Item] {
        if self.done {
            return .None
        }

        let item = self.inner.next();
        if item.isNone() {
            self.done = true;
            return .None
        }

        let value = item.unwrap();
        if self.predicate(value) {
            .Some(value)
        } else {
            self.done = true;
            .None
        }
    }
}

// ZipIterator - pairs elements from two iterators
public struct ZipIterator[A, B]: Iterator where A: Iterator, B: Iterator {
    type Item = (A.Item, B.Item)

    private var first: A
    private var second: B

    public init(first: A, second: B) {
        self.first = first;
        self.second = second;
    }

    public mutating func next() -> Optional[(A.Item, B.Item)] {
        let a = self.first.next();
        if a.isNone() {
            return .None
        }
        let b = self.second.next();
        if b.isNone() {
            return .None
        }
        let pair = (a.unwrap(), b.unwrap());
        .Some(pair)
    }
}

// EnumerateIterator - yields (index, item) pairs
public struct EnumerateIterator[I]: Iterator where I: Iterator {
    type Item = (Int64, I.Item)

    private var inner: I
    private var index: Int64

    public init(inner: I) {
        self.inner = inner;
        self.index = Int64(intLiteral: 0);
    }

    public mutating func next() -> Optional[(Int64, I.Item)] {
        let item = self.inner.next();
        if item.isSome() {
            let i = self.index;
            self.index = self.index + Int64(intLiteral: 1);
            .Some((i, item.unwrap()))
        } else {
            .None
        }
    }
}

// SkipWhileIterator - skips elements while predicate is true
public struct SkipWhileIterator[I]: Iterator where I: Iterator {
    type Item = I.Item

    private var inner: I
    private var predicate: (I.Item) -> Bool
    private var doneSkipping: Bool

    public init(inner: I, predicate: (I.Item) -> Bool) {
        self.inner = inner;
        self.predicate = predicate;
        self.doneSkipping = false;
    }

    public mutating func next() -> Optional[I.Item] {
        if self.doneSkipping {
            return self.inner.next()
        }

        // Skip while predicate is true
        var found: Bool = false;
        var result: Optional[I.Item] = .None;
        while found == false {
            let item = self.inner.next();
            if item.isNone() {
                self.doneSkipping = true;
                found = true
            } else {
                let value = item.unwrap();
                if self.predicate(value) == false {
                    self.doneSkipping = true;
                    result = .Some(value);
                    found = true
                }
            }
        }
        result
    }
}

// TakeIterator - takes first n elements
public struct TakeIterator[I]: Iterator where I: Iterator {
    type Item = I.Item

    private var inner: I
    private var remaining: Int64

    public init(inner: I, count: Int64) {
        self.inner = inner;
        self.remaining = count;
    }

    public mutating func next() -> Optional[I.Item] {
        if self.remaining > Int64(intLiteral: 0) {
            self.remaining = self.remaining - Int64(intLiteral: 1);
            self.inner.next()
        } else {
            .None
        }
    }
}

// SkipIterator - skips first n elements
public struct SkipIterator[I]: Iterator where I: Iterator {
    type Item = I.Item

    private var inner: I
    private var remaining: Int64

    public init(inner: I, count: Int64) {
        self.inner = inner;
        self.remaining = count;
    }

    public mutating func next() -> Optional[I.Item] {
        // Skip remaining elements first
        while self.remaining > Int64(intLiteral: 0) {
            let item = self.inner.next();
            if item.isNone() {
                return .None
            }
            self.remaining = self.remaining - Int64(intLiteral: 1)
        }
        self.inner.next()
    }
}

// ChainIterator - chains two iterators together
public struct ChainIterator[A, B]: Iterator where A: Iterator, B: Iterator, B.Item = A.Item {
    type Item = A.Item

    private var first: A
    private var second: B
    private var firstDone: Bool

    public init(first: A, second: B) {
        self.first = first;
        self.second = second;
        self.firstDone = false;
    }

    public mutating func next() -> Optional[A.Item] {
        if not self.firstDone {
            let item = self.first.next();
            if item.isSome() {
                return item
            }
            self.firstDone = true
        }
        self.second.next()
    }
}

// PeekableIterator - allows peeking at next element without consuming
public struct PeekableIterator[I]: Iterator where I: Iterator {
    type Item = I.Item

    private var inner: I
    private var peeked: Optional[Optional[I.Item]]

    public init(inner: I) {
        self.inner = inner;
        self.peeked = .None;
    }

    public mutating func peek() -> Optional[I.Item] {
        if self.peeked.isNone() {
            self.peeked = .Some(self.inner.next())
        }
        self.peeked.unwrap()
    }

    public mutating func next() -> Optional[I.Item] {
        if self.peeked.isSome() {
            let result = self.peeked.unwrap();
            self.peeked = .None;
            return result
        }
        self.inner.next()
    }
}

// CycleIterator - repeats iterator forever
public struct CycleIterator[I]: Iterator where I: Iterator, I: Cloneable {
    type Item = I.Item

    private var original: I
    private var current: I

    public init(iter: I) {
        self.original = iter.clone();
        self.current = iter;
    }

    public mutating func next() -> Optional[I.Item] {
        let item = self.current.next();
        if item.isSome() {
            return item
        }
        self.current = self.original.clone();
        self.current.next()
    }
}

// FuseIterator - stops permanently after first None
public struct FuseIterator[I]: Iterator where I: Iterator {
    type Item = I.Item

    private var inner: I
    private var done: Bool

    public init(inner: I) {
        self.inner = inner;
        self.done = false;
    }

    public mutating func next() -> Optional[I.Item] {
        if self.done {
            return .None
        }

        let item = self.inner.next();
        if item.isNone() {
            self.done = true
        }
        item
    }
}

// EmptyIterator - yields nothing
public struct EmptyIterator[T]: Iterator {
    type Item = T

    public init() {}

    public mutating func next() -> Optional[T] {
        .None
    }
}

// OnceIterator - yields a single value
public struct OnceIterator[T]: Iterator {
    type Item = T

    private var value: Optional[T]

    public init(value: T) {
        self.value = .Some(value);
    }

    public mutating func next() -> Optional[T] {
        let result = self.value;
        self.value = .None;
        result
    }
}

// RepeatIterator - yields the same value forever
public struct RepeatIterator[T]: Iterator where T: Cloneable {
    type Item = T

    private var value: T

    public init(value: T) {
        self.value = value;
    }

    public mutating func next() -> Optional[T] {
        .Some(self.value.clone())
    }
}

// RepeatNIterator - yields the same value n times
public struct RepeatNIterator[T]: Iterator where T: Cloneable {
    type Item = T

    private var value: T
    private var remaining: Int64

    public init(value value: T, count count: Int64) {
        self.value = value;
        self.remaining = count;
    }

    public mutating func next() -> Optional[T] {
        if self.remaining > Int64(intLiteral: 0) {
            self.remaining = self.remaining - Int64(intLiteral: 1);
            .Some(self.value.clone())
        } else {
            .None
        }
    }
}

// Convenience functions
public func empty[T]() -> EmptyIterator[T] {
    EmptyIterator()
}

public func once[T](value: T) -> OnceIterator[T] {
    OnceIterator(value)
}

public func repeatValue[T](value: T) -> RepeatIterator[T] where T: Cloneable {
    RepeatIterator(value)
}

public func repeatN[T](value: T, count: Int64) -> RepeatNIterator[T] where T: Cloneable {
    RepeatNIterator(value: value, count: count)
}
