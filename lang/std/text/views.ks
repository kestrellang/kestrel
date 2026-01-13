// String views for different representations

module std.text

import std.core.(Equatable, Comparable, Int, Bool, Ordering)
import std.result.(Optional)
import std.memory.(Allocator, Slice)
import std.iter.(Iterator, Iterable)

// BytesView - raw UTF-8 bytes (O(1) indexing)
public struct BytesView[A]: Iterable where A: Allocator {
    type Item = Byte
    type Iter = BytesIterator[A]

    private var string: String[A]

    public init(string: String[A]) {
        self.string = string;
    }

    public var count: Int {
        self.string.byteCount
    }

    public var isEmpty: Bool {
        self.string.isEmpty
    }

    //public subscript(safe index: Int) -> Optional[Byte] {
    //    get {
    //        if index >= 0 and index < self.count {
    //            .Some(self.string.byteAt(index: index))
    //        } else {
    //            .None
    //        }
    //    }
    //}

    //public subscript(unchecked index: Int) -> Byte {
    //    get { self.string.byteAt(index: index) }
    //}

    //public subscript(safe range: Range[Int]) -> Optional[Slice[Byte]] {
    //    get {
    //        if range.start >= 0 and range.end <= self.count {
    //            // Return a view of the bytes
    //            .Some(Slice(pointer: self.string.storage.value.buffer.pointer.offset(by: range.start),
    //                       count: range.end - range.start))
    //        } else {
    //            .None
    //        }
    //    }
    //}

    public func asSlice() -> Slice[Byte] {
        Slice(pointer: self.string.storage.value.buffer.pointer, count: self.count)
    }

    public func iter() -> BytesIterator[A] {
        BytesIterator(string: self.string, index: 0)
    }
}

public struct BytesIterator[A]: Iterator where A: Allocator {
    type Item = Byte

    private var string: String[A]
    private var index: Int

    public init(string: String[A], index: Int) {
        self.string = string;
        self.index = index;
    }

    public mutating func next() -> Optional[Byte] {
        if self.index < self.string.byteCount {
            let byte = self.string.byteAt(index: self.index);
            self.index = self.index + 1;
            .Some(byte)
        } else {
            .None
        }
    }
}

// CodePointsView - Unicode code points (O(1) iteration, O(n) indexing)
public struct CodePointsView[A]: Iterable where A: Allocator {
    type Item = CodePoint
    type Iter = CodePointsIterator[A]

    private var string: String[A]

    public init(string: String[A]) {
        self.string = string;
    }

    public func iter() -> CodePointsIterator[A] {
        CodePointsIterator(string: self.string, byteIndex: 0)
    }

    // Count is O(n) - must decode all code points
    public func count() -> Int {
        var n = 0;
        /* for _ in self {
            n += 1
        } */
        n
    }
}

public struct CodePointsIterator[A]: Iterator where A: Allocator {
    type Item = CodePoint

    private var string: String[A]
    private var byteIndex: Int

    public init(string: String[A], byteIndex: Int) {
        self.string = string;
        self.byteIndex = byteIndex
    }

    public mutating func next() -> Optional[CodePoint] {
        if self.byteIndex >= self.string.byteCount {
            return .None
        }

        if let (cp, len) = decodeUtf8(bytes: self.string.bytes.asSlice(), at: self.byteIndex) {
            self.byteIndex = self.byteIndex + len;
            return .Some(cp)
        }

        // Invalid UTF-8 - skip byte and return replacement character
        self.byteIndex = self.byteIndex + 1;
        .Some(CodePoint(value: 0xFFFD))  // Replacement character
    }
}

// CharsView - Extended grapheme clusters (O(1) iteration, O(n) indexing)
// Note: Full grapheme cluster support requires Unicode segmentation tables
// This is a simplified implementation that treats each code point as a char
public struct CharsView[A]: Iterable where A: Allocator {
    type Item = Char
    type Iter = CharsIterator[A]

    private var string: String[A]

    public init(string: String[A]) {
        self.string = string;
    }

    public func iter() -> CharsIterator[A] {
        CharsIterator(codePointsIter: self.string.codePoints.iter())
    }

    // Count is O(n) - must process all grapheme clusters
    public var count: Int {
        var n = 0;
        /* for _ in self {
            n += 1
        } */
        n
    }
}

public struct CharsIterator[A]: Iterator where A: Allocator {
    type Item = Char

    private var codePointsIter: CodePointsIterator[A]

    public init(codePointsIter: CodePointsIterator[A]) {
        self.codePointsIter = codePointsIter
    }

    public mutating func next() -> Optional[Char] {
        // Simplified: treat each code point as a character
        // Full implementation would need grapheme cluster segmentation
        self.codePointsIter.next().map { (cp) in
            Char(codePoint: cp)
        }
    }
}

// LinesView - line iterator
public struct LinesView[A]: Iterable where A: Allocator {
    type Item = String[A]
    type Iter = LinesIterator[A]

    private var string: String[A]

    public init(string: String[A]) {
        self.string = string;
    }

    public func iter() -> LinesIterator[A] {
        LinesIterator(string: self.string, byteIndex: 0, done: false)
    }
}

public struct LinesIterator[A]: Iterator where A: Allocator {
    type Item = String[A]

    private var string: String[A]
    private var byteIndex: Int
    private var done: Bool

    public init(string: String[A], byteIndex: Int, done: Bool) {
        self.string = string;
        self.byteIndex = byteIndex;
        self.done = done;
    }

    public mutating func next() -> Optional[String[A]] {
        if self.done or self.byteIndex >= self.string.byteCount {
            return .None
        }

        let start = self.byteIndex;

        // Find next newline
        while self.byteIndex < self.string.byteCount {
            let byte = self.string.byteAt(index: self.byteIndex);
            if byte == 10 {  // \n
                let line = self.string.substringBytes(from: start, to: self.byteIndex);
                self.byteIndex = self.byteIndex + 1;
                return .Some(line)
            } else if byte == 13 {  // \r
                let line = self.string.substringBytes(from: start, to: self.byteIndex);
                self.byteIndex = self.byteIndex + 1;
                // Handle \r\n
                if self.byteIndex < self.string.byteCount and self.string.byteAt(index: self.byteIndex) == 10 {
                    self.byteIndex = self.byteIndex + 1
                }
                return .Some(line)
            }
            self.byteIndex = self.byteIndex + 1
        }

        // Last line (no trailing newline)
        if start < self.string.byteCount {
            self.done = true;
            return .Some(self.string.substringBytes(from: start, to: self.string.byteCount))
        }

        return .None
    }
}

// String index types for O(1) access after initial scan
public struct ByteIndex: Equatable, Comparable {
    public var value: Int

    public init(value: Int) {
        self.value = value;
    }

    public func equals(other: ByteIndex) -> Bool {
        self.value == other.value
    }

    public func compare(other: ByteIndex) -> Ordering {
        self.value.compare(other.value)
    }
}

public struct CodePointIndex: Equatable {
    public var byteOffset: Int

    public init(byteOffset: Int) {
        self.byteOffset = byteOffset
    }

    public func equals(other: CodePointIndex) -> Bool {
        self.byteOffset == other.byteOffset
    }
}

public struct CharIndex: Equatable {
    public var byteOffset: Int

    public init(byteOffset: Int) {
        self.byteOffset = byteOffset
    }

    public func equals(other: CharIndex) -> Bool {
        self.byteOffset == other.byteOffset
    }
}
