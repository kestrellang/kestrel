// StringSlice - read-only borrowed window into a String's UTF-8 bytes

module std.text

import std.core.(Bool, Equal, NotEqual, Equatable, Comparable, Ordering, Cloneable, Hashable, Hasher, fatalError)
import std.numeric.(Int64, UInt8)
import std.result.(Optional)
import std.memory.(Layout, Pointer, RawPointer, UnsafeSlice)
import std.iter.(Iterable)
import std.text.(Formattable, Formatter, FormatOptions, Char, Grapheme, decodeUtf8, encodeUtf8, String, StringBuilder, StringStorage, CharsIterator, ByteIndex, CharIndex, GraphemeIndex, Str, _bytesEqual, _bytesCompare)

// ============================================================================
// STRING INDEX PROTOCOL
// ============================================================================

/// Protocol for typed string indices. Each index wraps a pre-resolved
/// byte offset; the type tag determines what unit the index addresses
/// and what the subscript returns.
public protocol StringIndex: Equatable, Comparable {
    type Yield
    @sugarOnly
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

/// A read-only window into a `String`'s UTF-8 bytes. It borrows the
/// string: a slice can't outlive the string it came from, and it can't
/// be stored in a field or an array that would (E494/E509). Use
/// `.toOwned()` to keep a piece.
///
/// Making and narrowing a slice is free: no refcount, and the string's
/// buffer isn't pinned, so changing the string later doesn't copy it.
/// Every read goes through the string's current buffer. If the string
/// gets shorter than the slice while the slice is alive, the next read
/// panics instead of reading freed memory.
///
/// # Examples
///
/// ```
/// let s = "hello, world";
/// let slice = s.asSlice();
/// slice.bytes.count;             // 12
/// slice.toOwned();               // "hello, world"
/// ```
///
/// # Representation
///
/// `(base: &String, start: Int64, end: Int64)`. `start` and `end` are
/// byte offsets into the whole string, not into this slice.
public struct StringSlice: Str, Equatable, Equal[StringSlice], Equal[String], NotEqual[StringSlice], NotEqual[String], Comparable, Hashable, Cloneable, Formattable, Iterable {
    type Item = Char
    type TargetIterator = CharsIterator
    type Equal[StringSlice].Output = Bool
    type Equal[String].Output = Bool
    type NotEqual[StringSlice].Output = Bool
    type NotEqual[String].Output = Bool

    // Module-internal so `String.asSlice()` can build one (memberwise).
    let base: &String
    public let start: Int64
    public let end: Int64

    // -- Size ----------------------------------------------------------------

    /// True when the slice covers zero bytes.
    public var isEmpty: Bool { self.start >= self.end }

    // -- Narrowing -----------------------------------------------------------

    /// Returns a sub-slice covering `[newStart, newEnd)` relative to
    /// the source buffer (absolute byte offsets, not relative to this
    /// slice's start).
    public func subslice(from newStart: Int64, to newEnd: Int64) -> StringSlice {
        StringSlice(base: self.base, start: newStart, end: newEnd)
    }

    // -- Conversion ----------------------------------------------------------

    /// Copies just this slice's bytes into a new independent `String`.
    public func toOwned() -> String {
        if self.isEmpty {
            return String()
        }
        let ptr = self._rawPtr();
        String.fromBytesUnchecked(ptr.offset(by: self.start), (self.end - self.start))
    }

    // -- Internal helpers ----------------------------------------------------

    /// The string's current byte buffer. Panics when the string got
    /// shorter than this slice since it was made.
    func _rawPtr() -> Pointer[UInt8] {
        if self.end > self.base._byteCount() {
            fatalError("StringSlice: the string is shorter than the slice");
        }
        self.base._bytePtr()
    }

    /// The current address of this slice's first byte, as the raw pointer
    /// the text views read through. Re-read it on every access.
    func _bytes() -> lang.ptr[lang.i8] {
        lang.cast_ptr[_, lang.i8](self._rawPtr().offset(by: self.start).asRaw().raw)
    }

    func _readByte(at offset: Int64) -> UInt8 {
        unsafe self._rawPtr().offset(by: offset).read()
    }

    // -- Iteration -----------------------------------------------------------

    /// Iterates code points in this slice.
    public func iter() -> CharsIterator {
        CharsIterator(source: self, length: (self.end - self.start), byteIndex: 0)
    }

    // -- Str conformance -----------------------------------------------------

    /// Returns self — StringSlice is already a slice.
    public func asSlice() -> StringSlice { self }

    // -- Protocol conformances -----------------------------------------------

    public func format(mutating into writer: some Formatter, options: FormatOptions = FormatOptions.default()) {
        self.toOwned().format(into: writer, options)
    }

    public func isEqual(to other: StringSlice) -> Bool {
        let myLen = (self.end - self.start);
        let otherLen = (other.end - other.start);
        if myLen != otherLen { return false }
        if myLen == 0 { return true }
        _bytesEqual(
            a: self._rawPtr().offset(by: self.start),
            b: other._rawPtr().offset(by: other.start),
            n: myLen
        )
    }

    public func equal(to other: StringSlice) -> Bool {
        self.isEqual(to: other)
    }

    public func notEqual(to other: StringSlice) -> Bool {
        if self.isEqual(to: other) { false } else { true }
    }

    public func equal(to other: String) -> Bool {
        self.isEqual(to: other.asSlice())
    }

    public func notEqual(to other: String) -> Bool {
        if self.isEqual(to: other.asSlice()) { false } else { true }
    }

    public func compare(other: StringSlice) -> Ordering {
        let myLen = (self.end - self.start);
        let otherLen = (other.end - other.start);
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
        hasher.write(unsafe UnsafeSlice(pointer: self._rawPtr().offset(by: self.start), count: (self.end - self.start)))
    }

    public func clone() -> StringSlice {
        StringSlice(base: self.base, start: self.start, end: self.end)
    }
}
