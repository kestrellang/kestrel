// Iterator adapter types

module std.iter

import std.result.(Optional)
import std.core.(Cloneable)

// MapIterator
public struct MapIterator[I, U]: Iterator where I: Iterator {
    type Item = U

    private var inner: I
    private var transform: (I.Item) -> U

    public init(inner: I, transform: (I.Item) -> U) {
        self.inner = inner;
        self.transform = transform;
    }

    public mutating func next() -> Optional[U] {
        self.inner.next().map(self.transform)
    }
}

// FilterIterator
public struct FilterIterator[I]: Iterator where I: Iterator {
    type Item = I.Item

    private var inner: I
    private var predicate: (I.Item) -> Bool

    public init(inner: I, predicate: (I.Item) -> Bool) {
        self.inner = inner;
        self.predicate = predicate;
    }

    public mutating func next() -> Optional[I.Item] {
        while let item = self.inner.next() {
            if self.predicate(item) {
                return .Some(item)
            }
        }
        .None
    }
}

// FilterMapIterator
public struct FilterMapIterator[I, U]: Iterator where I: Iterator {
    type Item = U

    private var inner: I
    private var transform: (I.Item) -> Optional[U]

    public init(inner: I, transform: (I.Item) -> Optional[U]) {
        self.inner = inner;
        self.transform = transform;
    }

    public mutating func next() -> Optional[U] {
        while let item = self.inner.next() {
            if let result = self.transform(item) {
                return .Some(result)
            }
        }
        .None
    }
}

// FlatMapIterator
public struct FlatMapIterator[I, Inner]: Iterator where I: Iterator, Inner: Iterable {
    type Item = Inner.Item

    private var inner: I
    private var transform: (I.Item) -> Inner
    private var current: Optional[Inner.Iter]

    public init(inner: I, transform: (I.Item) -> Inner, current: Optional[Inner.Iter]) {
        self.inner = inner;
        self.transform = transform;
        self.current = current;
    }

    public mutating func next() -> Optional[Inner.Item] {
        while true {
            if let currentIter = self.current {
                if let item = currentIter.next() {
                    return .Some(item)
                }
                self.current = .None
            }

            if let outerItem = self.inner.next() {
                self.current = .Some(self.transform(outerItem).iter())
            } else {
                return .None
            }
        }
    }
}

// InspectIterator
public struct InspectIterator[I]: Iterator where I: Iterator {
    type Item = I.Item

    private var inner: I
    private var action: (I.Item) -> ()

    public init(inner: I, action: (I.Item) -> ()) {
        self.inner = inner;
        self.action = action;
    }

    public mutating func next() -> Optional[I.Item] {
        self.inner.next().map { (item) in
            self.action(item);
            item
        }
    }
}

// TakeIterator
public struct TakeIterator[I]: Iterator where I: Iterator {
    type Item = I.Item

    private var inner: I
    private var remaining: Int

    public init(inner: I, remaining: Int) {
        self.inner = inner;
        self.remaining = remaining;
    }

    public mutating func next() -> Optional[I.Item] {
        if self.remaining > 0 {
            self.remaining = self.remaining - 1;
            self.inner.next()
        } else {
            .None
        }
    }
}

// TakeWhileIterator
public struct TakeWhileIterator[I]: Iterator where I: Iterator {
    type Item = I.Item

    private var inner: I
    private var predicate: (I.Item) -> Bool
    private var done: Bool

    public init(inner: I, predicate: (I.Item) -> Bool, done: Bool) {
        self.inner = inner;
        self.predicate = predicate;
        self.done = done;
    }

    public mutating func next() -> Optional[I.Item] {
        if self.done {
            return .None
        }

        if let item = self.inner.next() {
            if self.predicate(item) {
                return .Some(item)
            }
            self.done = true;
        }
        .None
    }
}

// SkipIterator
public struct SkipIterator[I]: Iterator where I: Iterator {
    type Item = I.Item

    private var inner: I
    private var remaining: Int

    public init(inner: I, remaining: Int) {
        self.inner = inner;
        self.remaining = remaining;
    }

    public mutating func next() -> Optional[I.Item] {
        while self.remaining > 0 {
            if self.inner.next().isNone {
                return .None
            }
            self.remaining = self.remaining - 1
        }
        self.inner.next()
    }
}

// SkipWhileIterator
public struct SkipWhileIterator[I]: Iterator where I: Iterator {
    type Item = I.Item

    private var inner: I
    private var predicate: (I.Item) -> Bool
    private var done: Bool

    public init(inner: I, predicate: (I.Item) -> Bool, done: Bool) {
        self.inner = inner;
        self.predicate = predicate;
        self.done = done;
    }

    public mutating func next() -> Optional[I.Item] {
        if self.done {
            return self.inner.next()
        }

        while let item = self.inner.next() {
            if not self.predicate(item) {
                self.done = true;
                return .Some(item)
            }
        }
        .None
    }
}

// StepByIterator
public struct StepByIterator[I]: Iterator where I: Iterator {
    type Item = I.Item

    private var inner: I
    private var step: Int
    private var first: Bool

    public init(inner: I, step: Int, first: Bool) {
        self.inner = inner;
        self.step = step;
        self.first = first;
    }

    public mutating func next() -> Optional[I.Item] {
        if self.first {
            self.first = false;
            return self.inner.next()
        }

        /* for _ in 0..<(self.step - 1) {
            if self.inner.next().isNone {
                return .None
            }
        } */
        self.inner.next()
    }
}

// EnumerateIterator
public struct EnumerateIterator[I]: Iterator where I: Iterator {
    type Item = (Int, I.Item)

    private var inner: I
    private var index: Int

    public init(inner: I, index: Int) {
        self.inner = inner;
        self.index = index;
    }

    public mutating func next() -> Optional[(Int, I.Item)] {
        self.inner.next().map { (item) in
            let i = self.index;
            self.index = self.index + 1;
            (i, item)
        }
    }
}

// ZipIterator
public struct ZipIterator[A, B]: Iterator where A: Iterator, B: Iterator {
    type Item = (A.Item, B.Item)

    private var first: A
    private var second: B

    public init(first: A, second: B) {
        self.first = first;
        self.second = second;
    }

    public mutating func next() -> Optional[(A.Item, B.Item)] {
        if let a = self.first.next() {
            if let b = self.second.next() {
                return .Some((a, b))
            }
        }
        .None
    }
}

// ChainIterator
public struct ChainIterator[A, B]: Iterator
    where A: Iterator, B: Iterator, B.Item = A.Item
{
    type Item = A.Item

    private var first: A
    private var second: B
    private var firstDone: Bool

    public init(first: A, second: B, firstDone: Bool) {
        self.first = first;
        self.second = second;
        self.firstDone = firstDone
    }

    public mutating func next() -> Optional[A.Item] {
        if not self.firstDone {
            if let item = self.first.next() {
                return .Some(item)
            }
            self.firstDone = true
        }
        self.second.next()
    }
}

// CycleIterator
public struct CycleIterator[I]: Iterator where I: Iterator, I: Cloneable {
    type Item = I.Item

    private var original: I
    private var current: I

    public init(original: I, current: I) {
        self.original = original;
        self.current = current;
    }

    public mutating func next() -> Optional[I.Item] {
        if let item = self.current.next() {
            return .Some(item)
        }
        self.current = self.original.clone();
        self.current.next()
    }
}

// IntersperseIterator
public struct IntersperseIterator[I]: Iterator where I: Iterator, I.Item: Cloneable {
    type Item = I.Item

    private var inner: I
    private var separator: I.Item
    private var needsSeparator: Bool

    public init(inner: I, separator: I.Item, needsSeparator: Bool) {
        self.inner = inner;
        self.separator = separator;
        self.needsSeparator = needsSeparator
    }

    public mutating func next() -> Optional[I.Item] {
        if self.needsSeparator {
            self.needsSeparator = false;
            return .Some(self.separator.clone())
        }

        self.inner.next().map { (item) in
            self.needsSeparator = true;
            item
        }
    }
}

// PeekableIterator
public struct PeekableIterator[I]: Iterator where I: Iterator {
    type Item = I.Item

    private var inner: I
    private var peeked: Optional[Optional[I.Item]]

    public init(inner: I, peeked: Optional[Optional[I.Item]]) {
        self.inner = inner;
        self.peeked = peeked;
    }

    public func peek() -> Optional[I.Item] {
        if self.peeked.isNone {
            self.peeked = .Some(self.inner.next())
        }
        self.peeked.unwrap()
    }

    public mutating func next() -> Optional[I.Item] {
        if let p = self.peeked {
            self.peeked = .None;
            return p
        }
        self.inner.next()
    }

    public func nextIf(predicate: (I.Item) -> Bool) -> Optional[I.Item] {
        if let item = self.peek() {
            if predicate(item) {
                return self.next()
            }
        }
        .None
    }
}

// FuseIterator - stops permanently after first None
public struct FuseIterator[I]: Iterator where I: Iterator {
    type Item = I.Item

    private var inner: I
    private var done: Bool

    public init(inner: I, done: Bool) {
        self.inner = inner;
        self.done = done;
    }

    public mutating func next() -> Optional[I.Item] {
        if self.done {
            return .None
        }

        match self.inner.next() {
            .Some(item) => .Some(item),
            .None => {
                self.done = true;
                .None
            }
        }
    }
}

// EmptyIterator
public struct EmptyIterator[T]: Iterator {
    type Item = T

    public init() {}

    public mutating func next() -> Optional[T] {
        .None
    }
}

// OnceIterator
public struct OnceIterator[T]: Iterator {
    type Item = T

    private var value: Optional[T]

    public init(value: T) {
        self.value = .Some(value)
    }

    public mutating func next() -> Optional[T] {
        let result = self.value;
        self.value = .None;
        result
    }
}

// RepeatIterator
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

// RepeatNIterator
public struct RepeatNIterator[T]: Iterator where T: Cloneable {
    type Item = T

    private var value: T
    private var remaining: Int

    public init(value: T, count: Int) {
        self.value = value;
        self.remaining = count;
    }

    public mutating func next() -> Optional[T] {
        if self.remaining > 0 {
            self.remaining = self.remaining - 1;
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
    OnceIterator(value: value)
}

public func repeat[T](value: T) -> RepeatIterator[T] where T: Cloneable {
    RepeatIterator(value: value)
}

public func repeatN[T](value: T, count: Int) -> RepeatNIterator[T] where T: Cloneable {
    RepeatNIterator(value: value, count: count)
}
