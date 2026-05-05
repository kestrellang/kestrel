// StringSlice - immutable window into a String's UTF-8 bytes

module std.text

import std.core.(Bool, Equatable, Comparable, Ordering, Cloneable, Hashable, Hasher, fatalError)
import std.numeric.(Int64, UInt8)
import std.result.(Optional)
import std.memory.(Layout, Pointer, RawPointer, RcBox, ArraySlice)
import std.iter.(Iterable)
import std.text.(Formattable, FormatOptions, Char, Grapheme, decodeUtf8, encodeUtf8, String, StringBuilder, StringStorage, CharsIterator, ByteIndex, CharIndex, GraphemeIndex, Str, _bytesEqual, _bytesCompare)

// ============================================================================
// STRING INDEX PROTOCOL
// ============================================================================

/// Protocol for typed string indices. Each index wraps a pre-resolved
/// byte offset; the type tag determines what unit the index addresses
/// and what the subscript returns.
public protocol StringIndex: Equatable, Comparable {
    type Yield
    func read(from slice: StringSlice) -> Yield
}

// ============================================================================
// LINE INDEX
// ============================================================================

/// A typed wrapper for a line position within a string.
/// Stores the byte offset of the line's first byte.
public struct LineIndex: Equatable, Comparable {
    public var byteOffset: Int64

    public init(byteOffset: Int64) {
        self.byteOffset = byteOffset;
    }

    public func isEqual(to other: LineIndex) -> Bool {
        self.byteOffset == other.byteOffset
    }

    public func compare(other: LineIndex) -> Ordering {
        self.byteOffset.compare(other.byteOffset)
    }
}

// ============================================================================
// COMPARABLE EXTENSIONS FOR EXISTING INDEX TYPES
// ============================================================================

extend CharIndex: Comparable {
    public func compare(other: CharIndex) -> Ordering {
        self.byteOffset.compare(other.byteOffset)
    }
}

extend GraphemeIndex: Comparable {
    public func compare(other: GraphemeIndex) -> Ordering {
        self.byteOffset.compare(other.byteOffset)
    }
}

// ============================================================================
// INDEX ADVANCEMENT
// ============================================================================

extend ByteIndex {
    /// Advances by `n` bytes. Pure arithmetic — no string needed.
    public func advance(by n: Int64) -> ByteIndex {
        ByteIndex(self.value + n)
    }
}

extend CharIndex {
    /// Advances by `n` code points. Requires the source string to
    /// decode UTF-8 boundaries. O(n) in chars advanced.
    public func advance(by n: Int64, from source: StringSlice) -> CharIndex {
        let ptr = source._rawPtr();
        let end = source.end;
        var offset = self.byteOffset;
        var remaining = n;
        while remaining > 0 and offset < end {
            let byte = source._readByte(at: offset);
            if byte < 0x80 {
                offset = offset + 1
            } else if byte < 0xE0 {
                offset = offset + 2
            } else if byte < 0xF0 {
                offset = offset + 3
            } else {
                offset = offset + 4
            }
            remaining = remaining - 1
        }
        CharIndex(offset)
    }
}

// ============================================================================
// STRING SLICE
// ============================================================================

/// An immutable window into a `String`'s UTF-8 bytes with shared
/// ownership. The central read-only abstraction of the text library.
///
/// Zero-cost to create from a String (share the RcBox, cover the
/// whole range). Zero-cost to narrow (adjust start/end). Keeps the
/// source alive as long as the slice exists.
///
/// # Examples
///
/// ```
/// let s = "hello, world";
/// let slice = s.asSlice();
/// slice.byteCount;              // 12
/// slice.toOwned();               // "hello, world"
/// ```
///
/// # Representation
///
/// `(source: RcBox[StringStorage], start: Int64, end: Int64)`.
///
/// # Memory Model
///
/// Shared ownership via `RcBox`. The source string's buffer stays
/// alive as long as any slice references it. Call `.toOwned()` to
/// copy just the slice's bytes into an independent `String`.
public struct StringSlice: Str, Equatable, Comparable, Hashable, Cloneable, Formattable, Iterable {
    type Item = Char
    type TargetIterator = CharsIterator

    var source: RcBox[StringStorage]
    public var start: Int64
    public var end: Int64

    /// @name From Source
    /// Creates a slice covering `[start, end)` in the given storage.
    public init(source source: RcBox[StringStorage], start start: Int64, end end: Int64) {
        self.source = source;
        self.start = start;
        self.end = end;
    }

    // -- Size ----------------------------------------------------------------

    /// Number of UTF-8 bytes in this slice. O(1).
    public var byteCount: Int64 { self.end - self.start }

    /// True when the slice covers zero bytes.
    public var isEmpty: Bool { self.start >= self.end }

    // -- Narrowing -----------------------------------------------------------

    /// Returns a sub-slice covering `[newStart, newEnd)` relative to
    /// the source buffer (absolute byte offsets, not relative to this
    /// slice's start).
    public func subslice(from newStart: Int64, to newEnd: Int64) -> StringSlice {
        StringSlice(source: self.source.clone(), start: newStart, end: newEnd)
    }

    // -- Conversion ----------------------------------------------------------

    /// Copies just this slice's bytes into a new independent `String`.
    public func toOwned() -> String {
        if self.isEmpty {
            return String()
        }
        let ptr = self.source.getValue().ptr;
        String.fromBytesUnchecked(ptr.offset(by: self.start), self.byteCount)
    }

    // -- Internal helpers ----------------------------------------------------

    func _rawPtr() -> Pointer[UInt8] {
        self.source.getValue().ptr
    }

    func _readByte(at offset: Int64) -> UInt8 {
        self.source.getValue().ptr.offset(by: offset).read()
    }

    // -- Iteration -----------------------------------------------------------

    /// Iterates code points in this slice.
    public func iter() -> CharsIterator {
        let rawPtr: lang.ptr[lang.i8] = lang.cast_ptr[_, lang.i8](self._rawPtr().offset(by: self.start).asRaw().raw);
        CharsIterator(ptr: rawPtr, length: self.byteCount, byteIndex: 0)
    }

    // -- Str conformance -----------------------------------------------------

    /// Returns self — StringSlice is already a slice.
    public func asSlice() -> StringSlice { self }

    // -- Protocol conformances -----------------------------------------------

    public func format(mutating into writer: StringBuilder, options: FormatOptions = FormatOptions.default()) {
        self.toOwned().format(into: writer, options)
    }

    public func isEqual(to other: StringSlice) -> Bool {
        let myLen = self.byteCount;
        let otherLen = other.byteCount;
        if myLen != otherLen { return false }
        if myLen == 0 { return true }
        _bytesEqual(
            a: self._rawPtr().offset(by: self.start),
            b: other._rawPtr().offset(by: other.start),
            n: myLen
        )
    }

    public func compare(other: StringSlice) -> Ordering {
        let myLen = self.byteCount;
        let otherLen = other.byteCount;
        let minLen = if myLen < otherLen { myLen } else { otherLen };
        let cmp = _bytesCompare(
            a: self._rawPtr().offset(by: self.start),
            b: other._rawPtr().offset(by: other.start),
            n: minLen
        );
        if cmp != .Equal { return cmp }
        myLen.compare(otherLen)
    }

    public func hash[H](mutating into hasher: H) where H: Hasher {
        hasher.write(ArraySlice(pointer: self._rawPtr().offset(by: self.start), count: self.byteCount))
    }

    public func clone() -> StringSlice {
        StringSlice(source: self.source.clone(), start: self.start, end: self.end)
    }
}

