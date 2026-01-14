// String type - UTF-8 encoded string with COW semantics

module std.text

import std.core.(Equatable, Comparable, Hashable, Hasher, Cloneable, Ordering, UInt8, UInt64, Int, Bool)
import std.result.(Optional)
import std.memory.(Allocator, ArcBox, Buffer, Slice)
import std.collections.(Array)
import std.ops.(ExpressibleByStringLiteral, Addable)

public struct String[A]:
    ExpressibleByStringLiteral,
    Addable,
    Equatable,
    Comparable,
    Hashable,
    Cloneable
    where A: Allocator
{
    // Note: String is NOT Iterable - must use a view

    private var storage: ArcBox[StringStorage[A]]

    struct StringStorage[A1] where A1: Allocator {
        var buffer: Buffer[UInt8, A1]
        var length: Int  // byte length
    }

    // Constructors
    public init() {
        self.storage = ArcBox(value: StringStorage(
            buffer: Buffer(capacity: 0),
            length: 0
        ))
    }

    public init(allocator: A) {
        self.storage = ArcBox(value: StringStorage(
            buffer: Buffer(capacity: 0, allocator: allocator),
            length: 0
        ))
    }

    public init(capacity: Int) {
        self.storage = ArcBox(value: StringStorage(
            buffer: Buffer(capacity: capacity),
            length: 0
        ))
    }

    // ExpressibleByStringLiteral
    public init(stringLiteral value: String) {
        self = value
    }

    // From bytes (must be valid UTF-8)
    public init(utf8 bytes: Slice[UInt8]) {
        self.storage = ArcBox(value: StringStorage(
            buffer: Buffer(capacity: bytes.count),
            length: bytes.count
        ))
        /* for i in 0..<bytes.count {
            (self.storage.value.buffer)(unchecked: i) = bytes(unchecked: i)
        } */
    }

    // From code points
    public init(codePoints: Array[CodePoint, A]) {
        var bytes: [UInt8] = [];
        /* for cp in codePoints {
            cp.encodeUtf8(into: bytes)
        } */
        self.init(utf8: bytes.asSlice())
    }

    // Properties
    public var isEmpty: Bool {
        self.storage.value.length == 0
    }

    public var byteCount: Int {
        self.storage.value.length
    }

    // Views for different representations
    public var bytes: BytesView[A] {
        BytesView(string: self)
    }

    public var codePoints: CodePointsView[A] {
        CodePointsView(string: self)
    }

    public var chars: CharsView[A] {
        CharsView(string: self)
    }

    public var lines: LinesView[A] {
        LinesView(string: self)
    }

    // COW helper
    private mutating func ensureUnique() {
        if not self.storage.isUnique() {
            self.storage = self.storage.deepClone()
        }
    }

    private mutating func ensureCapacity(minCapacity: Int) {
        self.ensureUnique();
        if self.storage.value.buffer.capacity < minCapacity {
            let newCapacity = if self.storage.value.buffer.capacity == 0 {
                if minCapacity < 16 { 16 } else { minCapacity }
            } else {
                var cap = self.storage.value.buffer.capacity;
                while cap < minCapacity {
                    cap = cap * 2
                }
                cap
            };
            self.storage.value.buffer.resize(to: newCapacity);
        }
    }

    // Mutation
    public mutating func append(string other: String) {
        self.ensureCapacity(minCapacity: self.byteCount + other.byteCount)
        /* for i in 0..<other.byteCount {
            (self.storage.value.buffer)(unchecked: self.storage.value.length) = (other.storage.value.buffer)(unchecked: i)
            self.storage.value.length += 1
        } */
    }

    public mutating func append(codePoint cp: CodePoint) {
        var bytes: [UInt8] = [];
        cp.encodeUtf8(into: bytes);
        self.ensureCapacity(minCapacity: self.byteCount + bytes.count);
        /* for byte in bytes {
            (self.storage.value.buffer)(unchecked: self.storage.value.length) = byte
            self.storage.value.length += 1
        } */
    }

    public mutating func clear() {
        self.ensureUnique();
        self.storage.value.length = 0
    }

    // Addable
    type Output = String[A]

    public func add(other: String[A]) -> String[A] {
        var result = self.clone();
        result.append(string: other);
        result
    }

    // Search
    public func contains(substring: String) -> Bool {
        if substring.isEmpty { return true }
        if substring.byteCount > self.byteCount { return false }

        /* for i in 0..=(self.byteCount - substring.byteCount) {
            var found = true
            for j in 0..<substring.byteCount {
                if (self.storage.value.buffer)(unchecked: i + j) != (substring.storage.value.buffer)(unchecked: j) {
                    found = false
                    break
                }
            }
            if found { return true }
        } */
        false
    }

    public func starts(with prefix: String) -> Bool {
        if prefix.byteCount > self.byteCount { return false }
        /* for i in 0..<prefix.byteCount {
            if (self.storage.value.buffer)(unchecked: i) != (prefix.storage.value.buffer)(unchecked: i) {
                return false
            }
        } */
        true
    }

    public func ends(with suffix: String) -> Bool {
        if suffix.byteCount > self.byteCount { return false }
        let offset = self.byteCount - suffix.byteCount;
        /* for i in 0..<suffix.byteCount {
            if (self.storage.value.buffer)(unchecked: offset + i) != (suffix.storage.value.buffer)(unchecked: i) {
                return false
            }
        } */
        true
    }

    public func find(substring: String) -> Optional[Int] {
        if substring.isEmpty { return .Some(0) }
        if substring.byteCount > self.byteCount { return .None }

        /* for i in 0..=(self.byteCount - substring.byteCount) {
            var found = true
            for j in 0..<substring.byteCount {
                if (self.storage.value.buffer)(unchecked: i + j) != (substring.storage.value.buffer)(unchecked: j) {
                    found = false
                    break
                }
            }
            if found { return .Some(i) }
        } */
        return .None
    }

    // Transformation
    public func trim() -> String[A] {
        self.trimStart().trimEnd()
    }

    public func trimStart() -> String[A] {
        var start = 0;
        while start < self.byteCount {
            let byte = (self.storage.value.buffer)(unchecked: start);
            if byte == 32 or byte == 9 or byte == 10 or byte == 13 {
                start = start + 1
            } else {
                break
            }
        }
        self.substringBytes(from: start, to: self.byteCount)
    }

    public func trimEnd() -> String[A] {
        var end = self.byteCount;
        while end > 0 {
            let byte = (self.storage.value.buffer)(unchecked: end - 1);
            if byte == 32 or byte == 9 or byte == 10 or byte == 13 {
                end = end - 1
            } else {
                break
            }
        }
        self.substringBytes(from: 0, to: end)
    }

    public func lowercase() -> String[A] {
        var result = String[A](capacity: self.byteCount);
        /* for cp in self.codePoints {
            result.append(codePoint: cp.toLowercase())
        } */
        result
    }

    public func uppercase() -> String[A] {
        var result = String[A](capacity: self.byteCount);
        /* for cp in self.codePoints {
            result.append(codePoint: cp.toUppercase())
        } */
        result
    }

    public func replace(pattern: String, with replacement: String) -> String[A] {
        if pattern.isEmpty { return self.clone() }

        var result = String[A]();
        var i = 0;

        while i < self.byteCount {
            if i + pattern.byteCount <= self.byteCount {
                var found = true;
                /* for j in 0..<pattern.byteCount {
                    if (self.storage.value.buffer)(unchecked: i + j) != (pattern.storage.value.buffer)(unchecked: j) {
                        found = false;
                        break
                    }
                } */
                if found {
                    result.append(string: replacement);
                    i = i + pattern.byteCount;
                    continue
                }
            }
            // Append single byte (careful with UTF-8!)
            /* TODO: implement byte-by-byte appending with proper COW semantics
            (result.storage.value.buffer)(unchecked: result.storage.value.length) = (self.storage.value.buffer)(unchecked: i);
            result.storage.value.length = result.storage.value.length + 1;
            */
            i = i + 1
        }
        result
    }

    // Splitting
    public func split(on separator: String) -> SplitIterator[A] {
        SplitIterator(string: self, separator: separator, index: 0, done: false)
    }

    // Substring by byte indices (internal)
    private func substringBytes(from start: Int, to end: Int) -> String[A] {
        var result = String[A](capacity: end - start);
        /* for i in start..<end {
            (result.storage.value.buffer)(unchecked: result.storage.value.length) = (self.storage.value.buffer)(unchecked: i)
            result.storage.value.length += 1
        } */
        result
    }

    // Equatable
    public func equals(other: String[A]) -> Bool {
        if self.byteCount != other.byteCount { return false }
        /* for i in 0..<self.byteCount {
            if (self.storage.value.buffer)(unchecked: i) != (other.storage.value.buffer)(unchecked: i) {
                return false
            }
        } */
        true
    }

    // Comparable
    public func compare(other: String[A]) -> Ordering {
        let minLen = if self.byteCount < other.byteCount { self.byteCount } else { other.byteCount };
        /* for i in 0..<minLen {
            let a = (self.storage.value.buffer)(unchecked: i)
            let b = (other.storage.value.buffer)(unchecked: i)
            if a < b { return .Less }
            if a > b { return .Greater }
        } */
        if self.byteCount < other.byteCount { .Less }
        else if self.byteCount > other.byteCount { .Greater }
        else { .Equal }
    }

    // Hashable
    public func hash[H](mutating into hasher: H) where H: Hasher {
        /* for i in 0..<self.byteCount {
            hasher.write(bytes: [(self.storage.value.buffer)(unchecked: i)])
        } */
    }

    // Cloneable
    public func clone() -> String[A] {
        var result = String[A](capacity: self.byteCount);
        /* for i in 0..<self.byteCount {
            (result.storage.value.buffer)(unchecked: i) = (self.storage.value.buffer)(unchecked: i)
        } */
        result.storage.value.length = self.byteCount;
        result
    }

    // Raw byte access (for views)
    internal func byteAt(index: Int) -> UInt8 {
        (self.storage.value.buffer)(unchecked: index)
    }
}

// SplitIterator
public struct SplitIterator[A]: Iterator where A: Allocator {
    type Item = String[A]

    private var string: String[A]
    private var separator: String[A]
    private var index: Int
    private var done: Bool

    public init(string: String[A], separator: String[A], index: Int, done: Bool) {
        self.string = string;
        self.separator = separator;
        self.index = index;
        self.done = done;
    }

    public mutating func next() -> Optional[String[A]] {
        if self.done { return .None }

        if self.separator.isEmpty {
            // Empty separator - iterate code points
            if self.index >= self.string.byteCount {
                self.done = true;
                return .None
            }
            // Find next code point
            if let (cp, len) = decodeUtf8(bytes: self.string.bytes.asSlice(), at: self.index) {
                let result = self.string.substringBytes(from: self.index, to: self.index + len);
                self.index = self.index + len;
                return .Some(result)
            }
            self.done = true;
            return .None
        }

        var start = self.index;
        while self.index + self.separator.byteCount <= self.string.byteCount {
            var found = true;
            /* for j in 0..<self.separator.byteCount {
                if self.string.byteAt(index: self.index + j) != self.separator.byteAt(index: j) {
                    found = false
                    break
                }
            } */
            if found {
                let result = self.string.substringBytes(from: start, to: self.index);
                self.index = self.index + self.separator.byteCount;
                return .Some(result)
            }
            self.index = self.index + 1
        }

        // Remainder
        if start < self.string.byteCount {
            self.done = true;
            return .Some(self.string.substringBytes(from: start, to: self.string.byteCount))
        }

        self.done = true;
        .None
    }
}
