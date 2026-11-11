// StringSlice - an unsized, in-place view of UTF-8 text

module std.text

import std.core.(Bool, Equal, NotEqual, Equatable, Comparable, Ordering, Hashable, Hasher, fatalError)
import std.numeric.(Int64, UInt8)
import std.result.(Optional)
import std.memory.(Layout, Pointer, RawPointer)
import std.collections.(Slice)
import std.iter.(Iterable)
import std.text.(Formattable, Formatter, FormatOptions, Char, Grapheme, decodeUtf8, encodeUtf8, String, StringBuilder, CharsIterator, ByteIndex, CharIndex, GraphemeIndex, _bytesEqual, _bytesCompare)

// ============================================================================
// STRING INDEX PROTOCOL
// ============================================================================

/// Protocol for typed string indices. Each index wraps a pre-resolved
/// byte offset; the type tag determines what unit the index addresses
/// and what the subscript returns.
public protocol StringIndex: Equatable, Comparable {
    type Yield
    @sugarOnly
    func read(from slice: &StringSlice) -> Yield
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
    public func advance(by n: Int64, from source: &StringSlice) -> CharIndex {
        let end = source._len;
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

/// A read-only run of UTF-8 text, viewed in place.
///
/// `StringSlice` is unsized, like `Slice[T]`: you only ever hold one
/// behind a reference, `&StringSlice`, which is two words (the first
/// byte's address and the byte count). A `String` lends its whole text
/// as one (`s.asSlice()`), and every read-only text method lives here,
/// so `String` reaches them through its `Indirection` peel. Narrowing
/// (`trimmed()`, `split(by:)`, `subslice(from:to:)`) lends a smaller
/// slice of the same bytes; nothing is copied. Use `.toOwned()` to keep
/// a piece.
///
/// A slice doesn't own its bytes and doesn't keep the buffer alive. A
/// slice of a `String` is only valid until the string next changes
/// (`append`, `clear`, …) or is dropped; using it after that reads freed
/// memory. The compiler does not check this yet.
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
/// A fat pointer, `(first byte, byte count)`, over bytes that are always
/// valid UTF-8. Every byte offset std hands out for a slice counts from
/// its first byte.
public struct StringSlice: not Copyable {
    fileprivate var storage: lang.slice[UInt8]
}

extend StringSlice {
    /// Lends `count` bytes starting at `start` as a slice.
    ///
    /// # Safety
    ///
    /// The bytes must be valid UTF-8 and stay alive, unmoved and
    /// unchanged, for as long as the slice is used.
    static unsafe func lend(consuming start start: Pointer[UInt8], consuming count count: Int64) -> &StringSlice {
        let fat: lang.ptr[lang.slice[UInt8]] = lang.slice_from_parts(start.raw, count.raw);
        unsafe Pointer[StringSlice](raw: lang.cast_ptr[_, StringSlice](fat)).value
    }

    /// This slice's own fat pointer.
    fileprivate func fat() -> lang.ptr[lang.slice[UInt8]] {
        lang.cast_ptr[_, lang.slice[UInt8]](lang.ptr_to(self))
    }

    // -- Size ----------------------------------------------------------------

    /// The byte count, for std's byte loops (`bytes.count` publicly).
    var _len: Int64 { Int64(intLiteral: lang.slice_count(self.fat())) }

    /// True when the slice covers zero bytes.
    public var isEmpty: Bool { self._len == 0 }

    // -- Narrowing -----------------------------------------------------------

    /// Returns the sub-slice covering bytes `[from, to)` of this slice.
    /// Offsets count from this slice's first byte, like every byte
    /// position std hands out for a slice (`firstIndex(of:)`, `bytes`).
    /// Panics when the range doesn't fit.
    public func subslice(from newStart: Int64, to newEnd: Int64) -> &StringSlice {
        if newStart < 0 or newStart > newEnd or newEnd > self._len {
            fatalError("StringSlice: subslice range out of bounds");
        }
        unsafe StringSlice.lend(start: self._rawPtr().offset(by: newStart), count: newEnd - newStart)
    }

    /// Itself: a slice is already the in-place view.
    public func asSlice() -> &StringSlice { self }

    // -- Conversion ----------------------------------------------------------

    /// Copies just this slice's bytes into a new independent `String`.
    public func toOwned() -> String {
        if self.isEmpty {
            return String()
        }
        String.fromBytesUnchecked(self._rawPtr(), self._len)
    }

    // -- Internal helpers ----------------------------------------------------

    /// The address of this slice's first byte.
    func _rawPtr() -> Pointer[UInt8] { Pointer(raw: lang.cast_ptr[_, UInt8](self.fat())) }

    /// The first byte's address, as the raw pointer the text views read
    /// through.
    func _bytes() -> lang.ptr[lang.i8] { lang.cast_ptr[_, lang.i8](self.fat()) }

    func _readByte(at offset: Int64) -> UInt8 {
        unsafe self._rawPtr().offset(by: offset).read()
    }

    // -- Iteration -----------------------------------------------------------

    /// Iterates code points in this slice.
    public func iter() -> CharsIterator {
        CharsIterator(source: self, length: self._len, byteIndex: 0)
    }
}

// -- Protocol conformances ---------------------------------------------------

extend StringSlice: Iterable {
    type Item = Char
    type TargetIterator = CharsIterator
}

extend StringSlice: Formattable {
    public func format(mutating into writer: some Formatter, options: FormatOptions = FormatOptions.default()) {
        self.toOwned().format(into: writer, options)
    }
}

extend StringSlice: Equatable, Equal[StringSlice], Equal[String], NotEqual[StringSlice], NotEqual[String] {
    type Equal[StringSlice].Output = Bool
    type Equal[String].Output = Bool
    type NotEqual[StringSlice].Output = Bool
    type NotEqual[String].Output = Bool

    /// Byte-wise equality: not normalization-aware (see
    /// `equalsCaseInsensitive`).
    public func isEqual(to other: StringSlice) -> Bool {
        let n = self._len;
        if n != other._len { return false }
        if n == 0 { return true }
        _bytesEqual(a: self._rawPtr(), b: other._rawPtr(), n: n)
    }

    public func equal(to other: StringSlice) -> Bool { self.isEqual(to: other) }

    public func notEqual(to other: StringSlice) -> Bool { self.isEqual(to: other) == false }

    public func equal(to other: String) -> Bool { self.isEqual(to: other.asSlice()) }

    public func notEqual(to other: String) -> Bool { self.isEqual(to: other.asSlice()) == false }
}

extend StringSlice: Comparable {
    /// Lexicographic byte-wise order; a prefix sorts first.
    public func compare(other: StringSlice) -> Ordering {
        let myLen = self._len;
        let otherLen = other._len;
        let minLen = if myLen < otherLen { myLen } else { otherLen };
        let cmp = _bytesCompare(a: self._rawPtr(), b: other._rawPtr(), n: minLen);
        if cmp != .Equal { return cmp }
        myLen.compare(otherLen)
    }
}

extend StringSlice: Hashable {
    public func hash[H](mutating into hasher: H) where H: Hasher {
        hasher.write(self.asByteSlice());
        // Length terminator, as in `String.hash`.
        hasher.write(uint64: UInt64(truncating: self._len))
    }
}
