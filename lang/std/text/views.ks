// String views for different representations

module std.text

import std.core.(Bool, Equatable, Comparable, Ordering, Range, ClosedRange, RangeFrom, RangeUpTo, RangeThrough, Cloneable, fatalError)
import std.numeric.(Int64, UInt8, UInt32)
import std.result.(Optional)
import std.iter.(Iterator, Iterable)
import std.text.(Char, Grapheme, decodeUtf8, String, StringSlice, LineIndex)
import std.text.unicode.(GraphemeBreakProperty, graphemeBreakProperty, shouldBreakBetween)
import std.memory.(Pointer)
import std.collections.(Array)
import std.ffi.(memmem)

// ============================================================================
// BYTES VIEW
// ============================================================================

/// Single-pass forward iterator over the raw UTF-8 bytes of a string.
///
/// Yielded by `BytesView.iter()`. Walks the underlying buffer one byte
/// at a time and returns each as a `UInt8`. The iterator holds a raw
/// pointer into the source string's storage; do not mutate the source
/// while iterating.
///
/// # Examples
///
/// ```
/// var it = "hi".bytes.iter();
/// it.next();  // Some(104)  // 'h'
/// it.next();  // Some(105)  // 'i'
/// it.next();  // None
/// ```
///
/// # Representation
///
/// A `(ptr, length, index)` triple: a raw pointer to the buffer plus
/// the cursor and total-length pair the iterator advances through.
///
/// # Memory Model
///
/// Value type. The pointer aliases string storage; do not retain the
/// iterator across mutations of the source `String`.
public struct BytesIterator: Iterator {
    /// The element type yielded by `next()` — always `UInt8`.
    type Item = UInt8

    private var ptr: lang.ptr[lang.i8]
    private var length: Int64
    private var index: Int64

    /// @name From Pointer
    /// Constructs a bytes iterator from a raw pointer, total byte count, and starting offset.
    ///
    /// Prefer `String.bytes.iter()` over calling this directly.
    ///
    /// # Safety
    ///
    /// `ptr` must point to at least `length` valid bytes; `index` must
    /// be in `0..=length`.
    public init(ptr ptr: lang.ptr[lang.i8], length length: Int64, index index: Int64) {
        self.ptr = ptr;
        self.length = length;
        self.index = index;
    }

    /// Returns the next byte, or `None` once `index` reaches `length`.
    ///
    /// Each call reads one byte and advances the cursor by 1.
    public mutating func next() -> UInt8? {
        if self.index < self.length {
            let rawOffset: lang.i64 = self.index.raw;
            let bytePtr: lang.ptr[lang.i8] = lang.ptr_offset[lang.i8](self.ptr, rawOffset);
            let signedByte: lang.i8 = lang.ptr_read(bytePtr);
            self.index = self.index + 1;
            .Some(UInt8(raw: signedByte))
        } else {
            .None
        }
    }
}

/// A read-only view over the raw UTF-8 bytes of a `String`.
///
/// Returned by `String.bytes`. Provides O(1) byte indexing and
/// iteration; the bytes are returned as `UInt8` exactly as they sit
/// in memory. The most common reason to reach for `BytesView` is to
/// perform byte-level operations (substring searches, hashing) without
/// paying the cost of UTF-8 decoding. For code-point or grapheme
/// iteration, see `CharsView` / `GraphemesView`.
///
/// # Examples
///
/// ```
/// let s = "hi";
/// s.bytes.count;               // 2
/// s.bytes(checked: 0);         // Some(104)
/// s.bytes(checked: 5);         // None (out of bounds)
/// ```
///
/// # Representation
///
/// A `(ptr, length)` pair pointing at the source string's UTF-8 buffer.
///
/// # Memory Model
///
/// Borrows the source string's storage; the view is invalidated by any
/// mutation that reallocates that buffer. Copy out to a new `String`
/// (e.g. via `substring`) if you need an independent value.
public struct BytesView: Iterable, Cloneable {
    /// The element type yielded by iteration — always `UInt8`.
    type Item = UInt8
    /// The iterator type returned by `iter()`.
    type TargetIterator = BytesIterator

    fileprivate var slice: StringSlice
    fileprivate var ptr: lang.ptr[lang.i8]
    fileprivate var length: Int64

    /// @name From Slice
    /// Constructs a bytes view backed by the given string slice.
    /// The view retains shared ownership of the underlying bytes.
    public init(slice slice: StringSlice) {
        self.slice = slice;
        self.ptr = lang.cast_ptr[_, lang.i8](slice._rawPtr().offset(by: slice.start).asRaw().raw);
        self.length = slice.byteCount;
    }

    public func clone() -> BytesView { BytesView(slice: self.slice.clone()) }

    /// Number of bytes in the view.
    ///
    /// O(1). This is **byte** count, not character count — see
    /// `CharsView.count` for the latter (which is O(n)).
    public var count: Int64 { self.length }

    /// `true` if the view spans zero bytes.
    public var isEmpty: Bool { self.length == 0 }

    /// Returns the raw pointer to the underlying byte buffer.
    ///
    /// Intended for FFI bridges; the pointer is only valid as long as
    /// the source string remains live and unmutated.
    public func asRaw() -> lang.ptr[lang.i8] { self.ptr }

    /// @name Indexed Byte / Sub-view
    /// Reads a single byte (`UInt8`) for `Int64` indexes, or a zero-copy
    /// sub-view (`BytesView`) for `Range[Int64]` / `ClosedRange[Int64]`.
    /// Panics on out-of-bounds. Range slicing does not validate UTF-8
    /// boundaries — call `.toString()` on the sub-view if you need an
    /// owned `String` (which validates).
    public subscript[I](index: I) -> I.BytesYield where I: BytesIndex {
        get { index.readBytes(from: self) }
    }

    /// @name Checked Index
    /// Reads at `index`, returning `.None` on out-of-bounds.
    public subscript[I](checked index: I) -> I.BytesYield? where I: BytesIndex {
        get { index.readBytesChecked(from: self) }
    }

    /// @name Unchecked Index
    /// Reads at `index` with no bounds check.
    ///
    /// # Safety
    ///
    /// Caller must guarantee `0 <= index < count`. For ranges, the
    /// endpoints must be in `0..=count`; otherwise the resulting
    /// sub-view aliases out-of-bounds memory.
    public subscript[I](unchecked index: I) -> I.BytesYield where I: BytesIndex {
        get { index.readBytesUnchecked(from: self) }
    }

    /// @name Clamping
    /// Reads at `index` with bounds saturated to `[0, count)`. Single-
    /// byte indexes yield `UInt8?` (`None` on empty view); range indexes
    /// yield `BytesView` (always valid, possibly empty).
    public subscript[I](clamped index: I) -> I.BytesClampedYield where I: BytesClampable {
        get { index.readBytesClamped(from: self) }
    }

    /// @name Wrapping
    /// Reads at `index` with modulo wrap-around. Negative indices wrap
    /// from the end: `view.bytes(wrapped: -1)` reads the last byte.
    /// Returns `None` on an empty view.
    public subscript[I](wrapped index: I) -> I.BytesWrappedYield where I: BytesWrappable {
        get { index.readBytesWrapped(from: self) }
    }

    /// Returns a `BytesIterator` positioned at byte 0.
    ///
    /// Required by `Iterable`. Each call produces a fresh iterator —
    /// the view is reusable.
    public func iter() -> BytesIterator {
        BytesIterator(ptr: self.ptr, length: self.length, index: 0)
    }

    /// Internal: read a single byte at `index` without bounds check.
    /// Used by the subscript conformances and substring helpers.
    fileprivate func _readByteRaw(index index: Int64) -> UInt8 {
        let rawOffset: lang.i64 = index.raw;
        let bytePtr: lang.ptr[lang.i8] = lang.ptr_offset[lang.i8](self.ptr, rawOffset);
        let signedByte: lang.i8 = lang.ptr_read(bytePtr);
        UInt8(raw: signedByte)
    }

    /// Materializes the view as an owned `String`. Copies all bytes
    /// into a fresh buffer; the result is independent of the source.
    /// Bytes are copied verbatim — no UTF-8 validation is performed.
    public func toString() -> String {
        _copyByteRange(ptr: self.ptr, startByte: 0, endByte: self.length)
    }

    /// Convenience: dispatches to a `BytesSubstringIndex` to produce
    /// an owned `String` covering the requested byte range. Equivalent
    /// to `self(range).toString()` for both `Range[Int64]` and
    /// `ClosedRange[Int64]`.
    public func substring(range: some BytesSubstringIndex) -> String {
        range.readBytesSubstring(from: self)
    }

    /// Internal: build a sub-view over byte range `[startByte, endByte)`.
    fileprivate func _subView(startByte startByte: Int64, endByte endByte: Int64) -> BytesView {
        BytesView(slice: self.slice.subslice(from: self.slice.start + startByte, to: self.slice.start + endByte))
    }
}

// ============================================================================
// CHARS VIEW
// ============================================================================

/// Single-pass forward iterator over Unicode code points (`Char`).
///
/// Yielded by `CharsView.iter()` and consumed by `GraphemesIterator`.
/// On each `next()` call, decodes one UTF-8 character starting at the
/// current cursor and advances by its byte length. Invalid bytes are
/// skipped one at a time and surfaced as `U+FFFD` (the Unicode
/// replacement character) so the iteration always makes progress.
///
/// # Examples
///
/// ```
/// var it = "hi".chars.iter();
/// it.next();  // Some('h')
/// it.next();  // Some('i')
/// it.next();  // None
/// ```
///
/// # Representation
///
/// A `(ptr, length, byteIndex)` triple. `byteIndex` walks the buffer
/// in variable-width steps according to the UTF-8 encoding.
///
/// # Memory Model
///
/// Value type that aliases the source string's buffer. Do not retain
/// across mutations of the source `String`.
public struct CharsIterator: Iterator {
    /// The element type yielded by `next()` — always `Char`.
    type Item = Char

    private var ptr: lang.ptr[lang.i8]
    private var length: Int64
    private var byteIndex: Int64

    /// @name From Pointer
    /// Constructs a chars iterator from a raw pointer, byte length, and starting byte offset.
    ///
    /// Prefer `String.chars.iter()` over calling this directly.
    ///
    /// # Safety
    ///
    /// `ptr` must point to `length` valid UTF-8 bytes; `byteIndex` must
    /// be `0` or land on a UTF-8 boundary.
    public init(ptr ptr: lang.ptr[lang.i8], length length: Int64, byteIndex byteIndex: Int64) {
        self.ptr = ptr;
        self.length = length;
        self.byteIndex = byteIndex;
    }

    /// Returns the next code point, or `None` when the buffer is exhausted.
    ///
    /// On invalid UTF-8 the iterator yields the replacement character
    /// `U+FFFD` and advances by one byte; this guarantees forward
    /// progress without aborting.
    public mutating func next() -> Char? {
        if self.byteIndex >= self.length {
            return .None
        }

        let result = decodeUtf8(self.ptr, self.length, at: self.byteIndex);
        if let .Some(decoded) = result {
            self.byteIndex = self.byteIndex + decoded.bytesConsumed;
            .Some(decoded.char)
        } else {
            // Invalid UTF-8 - skip byte and return replacement character
            self.byteIndex = self.byteIndex + 1;
            let replacementValue = UInt32(raw: 0xFFFD);
            .Some(Char(unchecked: replacementValue))
        }
    }
}

/// A view over the Unicode code points in a `String`.
///
/// Returned by `String.chars`. Iteration is O(1) per code point but
/// `count()` is O(n) because UTF-8 is variable-width. Range subscripts
/// are O(n) (the segment-walk dominates) but yield a zero-copy
/// `CharsView` sub-view — call `.toString()` to materialize an owned
/// `String`. To index in O(1), use `BytesView` and convert byte offsets
/// back yourself.
///
/// # Examples
///
/// ```
/// let s = "héllo";
/// s.chars.count;                       // 5 (code points)
/// s.bytes.count;                       // 6 (bytes — 'é' is 2 bytes)
/// s.chars(0..<4).toString();           // "héll"
/// s.chars.substring(0..<4);            // "héll"
/// ```
///
/// # Representation
///
/// A `(ptr, length)` pair, plus the on-demand UTF-8 decoder.
///
/// # Memory Model
///
/// Borrows the source string's buffer. Invalidated by any mutation
/// that reallocates the storage.
public struct CharsView: Iterable, Cloneable {
    /// The element type yielded by iteration — always `Char`.
    type Item = Char
    /// The iterator type returned by `iter()`.
    type TargetIterator = CharsIterator

    fileprivate var slice: StringSlice
    fileprivate var ptr: lang.ptr[lang.i8]
    fileprivate var length: Int64

    /// @name From Slice
    /// Constructs a chars view backed by the given string slice.
    /// The view retains shared ownership of the underlying bytes.
    public init(slice slice: StringSlice) {
        self.slice = slice;
        self.ptr = lang.cast_ptr[_, lang.i8](slice._rawPtr().offset(by: slice.start).asRaw().raw);
        self.length = slice.byteCount;
    }

    public func clone() -> CharsView { CharsView(slice: self.slice.clone()) }

    /// Returns a `CharsIterator` positioned at byte 0.
    ///
    /// Each call returns a fresh iterator; the view itself is reusable.
    public func iter() -> CharsIterator {
        CharsIterator(ptr: self.ptr, length: self.length, byteIndex: 0)
    }

    /// `true` when the view spans zero bytes (no code points).
    ///
    /// O(1) — checks `byteCount`, not `count`.
    public var isEmpty: Bool { self.length == 0 }

    /// Number of code points. **O(n)** — walks the buffer counting
    /// UTF-8 leading bytes (those whose top two bits are not `10`). For
    /// ASCII strings this equals `byteCount`. Cache the result if you
    /// need it more than once; each access re-walks the string.
    // TODO: replace lang.i32_*/lang.cast_i8_i32/lang.ptr_* intrinsics in
    // byte-classification code with UInt8/RawPointer wrappers after LLVM switch
    public var count: Int64 {
        var n: Int64 = 0;
        for i in 0..<self.length {
            // Count leading bytes only (not continuation bytes 10xxxxxx)
            let rawOffset: lang.i64 = i.raw;
            let bytePtr: lang.ptr[lang.i8] = lang.ptr_offset[lang.i8](self.ptr, rawOffset);
            let signedByte: lang.i8 = lang.ptr_read(bytePtr);
            let byteVal: lang.i32 = lang.cast_i8_i32(signedByte);
            if lang.i32_ne(lang.i32_and(byteVal, 0xC0), 0x80) {
                n = n + 1
            }
        }
        n
    }

    /// @name Indexed Char / Sub-view
    /// Reads a single code point (`Char`) for `Int64` indexes, or a
    /// zero-copy `CharsView` sub-view for `Range[Int64]` /
    /// `ClosedRange[Int64]`. All access is O(n) because UTF-8 is
    /// variable-width. Panics on out-of-bounds.
    public subscript[I](index: I) -> I.CharsYield where I: CharsIndex {
        get { index.readChars(from: self) }
    }

    /// @name Checked Index
    /// Reads at `index`, returning `.None` on out-of-bounds.
    public subscript[I](checked index: I) -> I.CharsYield? where I: CharsIndex {
        get { index.readCharsChecked(from: self) }
    }

    /// @name Clamping
    /// Reads at `index` with bounds saturated to `[0, count)`. Single-
    /// char indexes yield `Char?` (`None` on empty view); range indexes
    /// yield `CharsView` (always valid, possibly empty).
    public subscript[I](clamped index: I) -> I.CharsClampedYield where I: CharsClampable {
        get { index.readCharsClamped(from: self) }
    }

    /// @name Wrapping
    /// Reads at `index` with modulo wrap-around. Negative indices wrap
    /// from the end: `view.chars(wrapped: -1)` reads the last char.
    /// Returns `None` on an empty view.
    public subscript[I](wrapped index: I) -> I.CharsWrappedYield where I: CharsWrappable {
        get { index.readCharsWrapped(from: self) }
    }

    /// Internal: walk the buffer to find the byte offset of code-point
    /// index `charIndex`. Returns the byte offset and `true` if found,
    /// or `(byteIndex, false)` for an out-of-range character index.
    /// Special cases: `charIndex == 0` returns `(0, true)`;
    /// `charIndex == count` returns `(length, true)`.
    fileprivate func _byteOffsetForCharIndex(charIndex charIndex: Int64) -> (Int64, Bool) {
        if charIndex < 0 {
            return (0, false)
        }
        if charIndex == 0 {
            return (0, true)
        }
        var ci: Int64 = 0;
        var bi: Int64 = 0;
        while bi < self.length {
            let result = decodeUtf8(self.ptr, self.length, at: bi);
            if let .Some(decoded) = result {
                ci = ci + 1;
                bi = bi + decoded.bytesConsumed;
                if ci == charIndex {
                    return (bi, true)
                }
            } else {
                bi = bi + 1
            }
        }
        // charIndex == count is a valid endpoint (one-past-the-end).
        if ci == charIndex {
            (bi, true)
        } else {
            (bi, false)
        }
    }

    /// Internal: build a sub-view over byte range `[startByte, endByte)`.
    fileprivate func _subView(startByte startByte: Int64, endByte endByte: Int64) -> CharsView {
        CharsView(slice: self.slice.subslice(from: self.slice.start + startByte, to: self.slice.start + endByte))
    }

    /// Materializes the view as an owned `String`. O(n) — copies bytes.
    public func toString() -> String {
        _copyByteRange(ptr: self.ptr, startByte: 0, endByte: self.length)
    }

    /// Convenience: dispatches to a `CharsSubstringIndex` to produce
    /// an owned `String` covering the requested code-point range.
    /// Equivalent to `self(range).toString()` for both `Range[Int64]`
    /// and `ClosedRange[Int64]`.
    public func substring(range: some CharsSubstringIndex) -> String {
        range.readCharsSubstring(from: self)
    }
}

// ============================================================================
// GRAPHEMES VIEW (UAX #29)
// ============================================================================

/// Iterator over extended grapheme clusters under UAX #29 segmentation.
///
/// Wraps a `CharsIterator` and consults the Unicode grapheme-break
/// property tables on each step. Buffers one look-ahead `Char` so it
/// can decide whether the next code point starts a new cluster; that
/// pending char is yielded as the start of the *next* cluster on the
/// following call. Handles ZWJ-joined sequences and regional-indicator
/// flag pairs.
///
/// # Examples
///
/// ```
/// var it = "a\u{0301}b".graphemes.iter();
/// it.next();  // Some(Grapheme: ['a', U+0301])
/// it.next();  // Some(Grapheme: ['b'])
/// it.next();  // None
/// ```
///
/// # Representation
///
/// Wraps a `CharsIterator` plus a small amount of state machine: the
/// pending look-ahead char, the previous break property, the
/// "previous-previous was Regional Indicator" flag (for flag pairs),
/// and a `started` marker.
public struct GraphemesIterator: Iterator {
    /// The element type yielded by `next()` — always `Grapheme`.
    type Item = Grapheme

    private var charsIter: CharsIterator
    private var pendingChar: Char?

    /// @name From Chars
    /// Wraps a `CharsIterator` to produce graphemes via UAX #29 segmentation.
    ///
    /// Prefer `someString.graphemes.iter()` over calling this directly.
    public init(charsIter: CharsIterator) {
        self.charsIter = charsIter;
        self.pendingChar = .None;
    }

    /// Returns the next grapheme cluster, or `None` when the source is exhausted.
    ///
    /// Accumulates code points until `shouldBreakBetween` reports a
    /// boundary, then returns them as a `Grapheme`. The look-ahead
    /// char that triggered the break is held back for the next call.
    public mutating func next() -> Grapheme? {
        var chars = Array[Char]();

        // Get first char (either pending or from iterator)
        var firstChar: Char? = .None;
        if let .Some(pending) = self.pendingChar {
            firstChar = .Some(pending);
            self.pendingChar = .None
        } else {
            firstChar = self.charsIter.next()
        }

        // If no first char, we're done
        if let .None = firstChar {
            return .None
        }

        let first = match firstChar {
            .Some(c) => c,
            .None => { return .None }
        };
        chars.append(first);

        var prevProp = graphemeBreakProperty(first);
        var prevPrevWasRI: Bool = false;
        var prevWasZWJ: Bool = prevProp == GraphemeBreakProperty.ZWJ;

        // Keep accumulating chars until we hit a break
        while true {
            let nextChar = self.charsIter.next();
            if let .None = nextChar {
                // End of string - return what we have
                break
            }

            let next = match nextChar {
                .Some(c) => c,
                .None => { break }
            };

            let currProp = graphemeBreakProperty(next);

            // Check if we should break here
            if shouldBreakBetween(prevProp, currProp, prevPrevWasRI, prevWasZWJ) {
                // Save this char for next grapheme
                self.pendingChar = .Some(next);
                break
            }

            // No break - add to current cluster
            chars.append(next);

            // Update state for next iteration
            prevPrevWasRI = prevProp == GraphemeBreakProperty.RegionalIndicator;
            prevWasZWJ = currProp == GraphemeBreakProperty.ZWJ;
            prevProp = currProp
        }

        // Return the grapheme
        if chars.count == 1 {
            .Some(Grapheme(char: chars(unchecked: 0)))
        } else {
            .Some(Grapheme(chars: chars))
        }
    }
}

/// A view over the user-perceived characters (extended grapheme clusters) of a `String`.
///
/// Returned by `String.graphemes`. Use this — not `chars` — when you
/// need the unit a user thinks of as a single character: emoji
/// sequences, accented forms, country flags, etc. Both iteration and
/// `count()` are O(n) because each cluster requires consulting the
/// UAX #29 break tables.
///
/// # Examples
///
/// ```
/// let flag = "\u{1F1FA}\u{1F1F8}";  // 🇺🇸
/// flag.chars.count;        // 2 (regional indicators)
/// flag.graphemes.count;    // 1 (one flag)
/// ```
///
/// # Representation
///
/// A `(ptr, length)` pair; iteration is delegated to a wrapped
/// `CharsIterator` plus the UAX #29 segmenter state machine.
public struct GraphemesView: Iterable, Cloneable {
    /// The element type yielded by iteration — always `Grapheme`.
    type Item = Grapheme
    /// The iterator type returned by `iter()`.
    type TargetIterator = GraphemesIterator

    fileprivate var slice: StringSlice
    fileprivate var ptr: lang.ptr[lang.i8]
    fileprivate var length: Int64

    /// @name From Slice
    /// Constructs a graphemes view backed by the given string slice.
    /// The view retains shared ownership of the underlying bytes.
    public init(slice slice: StringSlice) {
        self.slice = slice;
        self.ptr = lang.cast_ptr[_, lang.i8](slice._rawPtr().offset(by: slice.start).asRaw().raw);
        self.length = slice.byteCount;
    }

    public func clone() -> GraphemesView { GraphemesView(slice: self.slice.clone()) }

    /// Returns a `GraphemesIterator` positioned at byte 0.
    public func iter() -> GraphemesIterator {
        GraphemesIterator(CharsIterator(ptr: self.ptr, length: self.length, byteIndex: 0))
    }

    /// `true` when the view spans zero bytes (no graphemes).
    ///
    /// O(1) — checks `byteCount`, not `count`.
    public var isEmpty: Bool { self.length == 0 }

    /// Number of grapheme clusters. **O(n)** — walks the entire string
    /// through the UAX #29 segmenter. Cache the result if you need it
    /// more than once; each access re-walks the string.
    public var count: Int64 {
        var n: Int64 = 0;
        for _ in self.iter() {
            n = n + 1
        }
        n
    }

    /// @name Indexed Grapheme / Sub-view
    /// `Int64` reads a single cluster; `Range[Int64]` /
    /// `ClosedRange[Int64]` yield a zero-copy `GraphemesView` sub-view
    /// covering those clusters. **O(n)** — walks the segmenter from the
    /// start. Panics on out-of-bounds.
    public subscript[I](index: I) -> I.GraphemesYield where I: GraphemesIndex {
        get { index.readGraphemes(from: self) }
    }

    /// @name Checked Index
    /// Reads at `index`, returning `.None` on out-of-bounds.
    public subscript[I](checked index: I) -> I.GraphemesYield? where I: GraphemesIndex {
        get { index.readGraphemesChecked(from: self) }
    }

    /// @name Clamping
    /// Reads at `index` saturated to `[0, count)`. Single-grapheme
    /// indexes yield `Grapheme?` (`.None` only when the view is empty);
    /// range indexes yield `GraphemesView` (always valid, possibly empty).
    public subscript[I](clamped index: I) -> I.GraphemesClampedYield where I: GraphemesClampable {
        get { index.readGraphemesClamped(from: self) }
    }

    /// @name Wrapping
    /// Reads at `index` with modulo wrap-around. Negative indices wrap
    /// from the end: `view.graphemes(wrapped: -1)` reads the last
    /// grapheme cluster. Returns `None` on an empty view.
    public subscript[I](wrapped index: I) -> I.GraphemesWrappedYield where I: GraphemesWrappable {
        get { index.readGraphemesWrapped(from: self) }
    }

    /// Internal: walk the segmenter to grapheme index `graphemeIndex`,
    /// summing the UTF-8 byte length of each cluster. Returns the byte
    /// offset of that grapheme's start and `true` if found, or
    /// `(byteIdx, false)` for an out-of-range index. `graphemeIndex == 0`
    /// returns `(0, true)`; `graphemeIndex == count` returns `(length, true)`.
    fileprivate func _byteOffsetForGraphemeIndex(graphemeIndex graphemeIndex: Int64) -> (Int64, Bool) {
        if graphemeIndex < 0 {
            return (0, false)
        }
        if graphemeIndex == 0 {
            return (0, true)
        }
        var gi: Int64 = 0;
        var byteIdx: Int64 = 0;
        var it = self.iter();
        while true {
            let next = it.next();
            if let .Some(g) = next {
                byteIdx = byteIdx + g.utf8Length();
                gi = gi + 1;
                if gi == graphemeIndex {
                    return (byteIdx, true)
                }
            } else {
                break
            }
        }
        // graphemeIndex == count is a valid endpoint (one-past-the-end).
        if gi == graphemeIndex {
            (byteIdx, true)
        } else {
            (byteIdx, false)
        }
    }

    /// Internal: build a sub-view over byte range `[startByte, endByte)`.
    fileprivate func _subView(startByte startByte: Int64, endByte endByte: Int64) -> GraphemesView {
        GraphemesView(slice: self.slice.subslice(from: self.slice.start + startByte, to: self.slice.start + endByte))
    }

    /// Materializes the view as an owned `String`. O(n) — copies bytes.
    public func toString() -> String {
        _copyByteRange(ptr: self.ptr, startByte: 0, endByte: self.length)
    }

    /// Convenience: dispatches to a `GraphemesSubstringIndex` to
    /// produce an owned `String` covering the requested cluster range.
    /// Equivalent to `self(range).toString()` for both `Range[Int64]`
    /// and `ClosedRange[Int64]`.
    public func substring(range: some GraphemesSubstringIndex) -> String {
        range.readGraphemesSubstring(from: self)
    }
}

// ============================================================================
// LINES VIEW
// ============================================================================

/// Iterator that yields each line of a string as a `String`.
///
/// Recognises both `\n` (LF) and `\r\n` (CRLF) as line terminators
/// and a lone `\r` (CR) as a terminator on its own. The terminator
/// itself is **not** included in the yielded line. A trailing line
/// without a terminator is still emitted; an empty input emits no
/// lines.
///
/// # Examples
///
/// ```
/// var it = "a\nb\r\nc".lines.iter();
/// it.next();  // Some("a")
/// it.next();  // Some("b")
/// it.next();  // Some("c")
/// it.next();  // None
/// ```
///
/// # Representation
///
/// A `(ptr, length, byteIndex, done)` quadruple. `done` flips to true
/// after the trailing-no-terminator case has been emitted, so further
/// calls return `None`.
public struct LinesIterator: Iterator {
    /// The element type yielded by `next()` — always `String`.
    type Item = String

    private var ptr: lang.ptr[lang.i8]
    private var length: Int64
    private var byteIndex: Int64
    private var done: Bool

    /// @name From Pointer
    /// Constructs a lines iterator from a raw pointer, total byte count, starting byte offset, and `done` flag.
    ///
    /// Prefer `someString.lines.iter()` over calling this directly.
    ///
    /// # Safety
    ///
    /// `ptr` must point to `length` valid UTF-8 bytes; `byteIndex`
    /// must be `0` or sit at a UTF-8 boundary.
    public init(ptr ptr: lang.ptr[lang.i8], length length: Int64, byteIndex byteIndex: Int64, done done: Bool) {
        self.ptr = ptr;
        self.length = length;
        self.byteIndex = byteIndex;
        self.done = done;
    }

    /// Returns the next line, or `None` once exhausted.
    ///
    /// Scans byte-by-byte for `\n` or `\r`, treating `\r\n` as a
    /// single terminator. The yielded string contains the bytes up
    /// to but not including the terminator.
    public mutating func next() -> String? {
        if self.done or self.byteIndex >= self.length {
            return .None
        }

        let start = self.byteIndex;

        // Find next newline
        var foundNewline: Bool = false;
        var lineEnd: Int64 = self.byteIndex;

        while self.byteIndex < self.length and foundNewline == false {
            let rawOffset: lang.i64 = self.byteIndex.raw;
            let bytePtr: lang.ptr[lang.i8] = lang.ptr_offset[lang.i8](self.ptr, rawOffset);
            let byte: lang.i8 = lang.ptr_read(bytePtr);
            let byteVal: lang.i32 = lang.cast_i8_i32(byte);
            let unsignedByte: lang.i32 = lang.i32_and(byteVal, 0xFF);

            if Bool(boolLiteral: lang.i32_eq(unsignedByte, 10)) {  // \n
                lineEnd = self.byteIndex;
                self.byteIndex = self.byteIndex + 1;
                foundNewline = true
            } else if Bool(boolLiteral: lang.i32_eq(unsignedByte, 13)) {  // \r
                lineEnd = self.byteIndex;
                self.byteIndex = self.byteIndex + 1;
                // Handle \r\n
                if self.byteIndex < self.length {
                    let nextOffset: lang.i64 = self.byteIndex.raw;
                    let nextBytePtr: lang.ptr[lang.i8] = lang.ptr_offset[lang.i8](self.ptr, nextOffset);
                    let nextByte: lang.i8 = lang.ptr_read(nextBytePtr);
                    let nextByteVal: lang.i32 = lang.cast_i8_i32(nextByte);
                    let nextUnsigned: lang.i32 = lang.i32_and(nextByteVal, 0xFF);
                    if Bool(boolLiteral: lang.i32_eq(nextUnsigned, 10)) {
                        self.byteIndex = self.byteIndex + 1
                    }
                }
                foundNewline = true
            } else {
                self.byteIndex = self.byteIndex + 1
            }
        }

        if foundNewline {
            // Return the line without newline characters
            return .Some(self.createSubstring(start, lineEnd))
        }

        // Last line (no trailing newline)
        if start < self.length {
            self.done = true;
            return .Some(self.createSubstring(start, self.length))
        }

        let none: String? = .None;
        none
    }

    /// Copies bytes `[start, end)` into a fresh `String` via `memcpy`.
    private func createSubstring(start: Int64, end: Int64) -> String {
        let count = end - start;
        if count <= 0 {
            return String()
        }
        let srcAt: lang.ptr[lang.i8] = lang.ptr_offset[lang.i8](self.ptr, start.raw);
        String.fromRawBytes(srcAt, count)
    }
}

/// A view over the lines of a `String`, split on `\n`, `\r\n`, or `\r`.
///
/// Returned by `String.lines`. The yielded strings (from iteration or
/// single-line subscripting) do not include the terminator; a trailing
/// line without a terminator is still emitted. Range subscripts
/// (`lines(0..<n)`) yield a zero-copy `LinesView` sub-view whose
/// underlying byte range still includes the original terminators —
/// iterating the sub-view round-trips the same line strings, and
/// `.toString()` reconstructs the original substring exactly.
///
/// # Examples
///
/// ```
/// var lines = Array[String]();
/// for line in "a\nb\nc".lines {
///     lines.append(line);
/// }
/// lines.count;  // 3
///
/// // Range subscript preserves terminators in the underlying bytes:
/// "a\r\nb\nc".lines(0..<2).toString();  // "a\r\nb\n"
/// ```
///
/// # Representation
///
/// A `(ptr, length)` pair pointing into the source string.
public struct LinesView: Iterable, Cloneable {
    /// The element type yielded by iteration — always `String`.
    type Item = String
    /// The iterator type returned by `iter()`.
    type TargetIterator = LinesIterator

    fileprivate var slice: StringSlice
    fileprivate var ptr: lang.ptr[lang.i8]
    fileprivate var length: Int64

    /// @name From Slice
    /// Constructs a lines view backed by the given string slice.
    /// The view retains shared ownership of the underlying bytes.
    public init(slice slice: StringSlice) {
        self.slice = slice;
        self.ptr = lang.cast_ptr[_, lang.i8](slice._rawPtr().offset(by: slice.start).asRaw().raw);
        self.length = slice.byteCount;
    }

    public func clone() -> LinesView { LinesView(slice: self.slice.clone()) }

    /// Returns a `LinesIterator` positioned at byte 0.
    public func iter() -> LinesIterator {
        LinesIterator(ptr: self.ptr, length: self.length, byteIndex: 0, done: false)
    }

    /// `true` when the view spans zero bytes (no lines).
    ///
    /// O(1) — checks `byteCount`, not `count`.
    public var isEmpty: Bool { self.length == 0 }

    /// Number of lines in the view. **O(n)** — walks the buffer
    /// scanning for terminators. Cache the result if you need it more
    /// than once.
    public var count: Int64 {
        var n: Int64 = 0;
        for _ in self.iter() {
            n = n + 1
        }
        n
    }

    /// @name Indexed Line / Sub-view
    /// `Int64` reads a single line as a `String` (without terminator).
    /// `Range[Int64]` / `ClosedRange[Int64]` yield a zero-copy
    /// `LinesView` sub-view covering those lines (terminators preserved
    /// in the underlying bytes). **O(n)** — walks the buffer from the
    /// start. Panics on out-of-bounds.
    public subscript[I](index: I) -> I.LinesYield where I: LinesIndex {
        get { index.readLines(from: self) }
    }

    /// @name Checked Index
    /// Reads at `index`, returning `.None` on out-of-bounds.
    public subscript[I](checked index: I) -> I.LinesYield? where I: LinesIndex {
        get { index.readLinesChecked(from: self) }
    }

    /// @name Clamping
    /// Reads at `index` saturated to `[0, count)`. Single-line indexes
    /// yield `String?` (`.None` only when the view holds no lines);
    /// range indexes yield `LinesView` (always valid, possibly empty).
    public subscript[I](clamped index: I) -> I.LinesClampedYield where I: LinesClampable {
        get { index.readLinesClamped(from: self) }
    }

    /// @name Wrapping
    /// Reads at `index` with modulo wrap-around. Negative indices wrap
    /// from the end: `view.lines(wrapped: -1)` reads the last line.
    /// Returns `None` on an empty view.
    public subscript[I](wrapped index: I) -> I.LinesWrappedYield where I: LinesWrappable {
        get { index.readLinesWrapped(from: self) }
    }

    /// Materializes the view as an owned `String` covering the entire
    /// underlying buffer (terminators included). O(n) — copies bytes.
    public func toString() -> String {
        _copyByteRange(ptr: self.ptr, startByte: 0, endByte: self.length)
    }

    /// Convenience: dispatches to a `LinesSubstringIndex` to produce
    /// an owned `String` covering the requested line range, with their
    /// original terminators preserved. Equivalent to
    /// `self(range).toString()` for both `Range[Int64]` and
    /// `ClosedRange[Int64]`.
    public func substring(range: some LinesSubstringIndex) -> String {
        range.readLinesSubstring(from: self)
    }

    /// Internal: walk the buffer to find the byte offset where line
    /// `lineIndex` begins. Mirrors `_byteOffsetForCharIndex` —
    /// `lineIndex == 0` returns `(0, true)`; `lineIndex == count`
    /// returns `(length, true)` so range endpoints can be one-past-end;
    /// negative or further-out indices return `(_, false)`. Recognises
    /// `\n`, `\r\n`, and lone `\r` terminators (matching `LinesIterator`).
    fileprivate func _byteOffsetForLineIndex(lineIndex lineIndex: Int64) -> (Int64, Bool) {
        if lineIndex < 0 {
            return (0, false)
        }
        if lineIndex == 0 {
            return (0, true)
        }
        var li: Int64 = 0;
        var bi: Int64 = 0;
        var lineStart: Int64 = 0;
        while bi < self.length {
            let rawOffset: lang.i64 = bi.raw;
            let bytePtr: lang.ptr[lang.i8] = lang.ptr_offset[lang.i8](self.ptr, rawOffset);
            let byte: lang.i8 = lang.ptr_read(bytePtr);
            let byteVal: lang.i32 = lang.cast_i8_i32(byte);
            let unsignedByte: lang.i32 = lang.i32_and(byteVal, 0xFF);
            if Bool(boolLiteral: lang.i32_eq(unsignedByte, 10)) {
                bi = bi + 1;
                li = li + 1;
                if li == lineIndex {
                    return (bi, true)
                }
                lineStart = bi
            } else if Bool(boolLiteral: lang.i32_eq(unsignedByte, 13)) {
                bi = bi + 1;
                if bi < self.length {
                    let nextOffset: lang.i64 = bi.raw;
                    let nextBytePtr: lang.ptr[lang.i8] = lang.ptr_offset[lang.i8](self.ptr, nextOffset);
                    let nextByte: lang.i8 = lang.ptr_read(nextBytePtr);
                    let nextByteVal: lang.i32 = lang.cast_i8_i32(nextByte);
                    let nextUnsigned: lang.i32 = lang.i32_and(nextByteVal, 0xFF);
                    if Bool(boolLiteral: lang.i32_eq(nextUnsigned, 10)) {
                        bi = bi + 1
                    }
                }
                li = li + 1;
                if li == lineIndex {
                    return (bi, true)
                }
                lineStart = bi
            } else {
                bi = bi + 1
            }
        }
        // Loop exited at end-of-buffer. If lineStart < length there is a
        // trailing line with no terminator — count it as line `li` so the
        // past-end endpoint (`lineIndex == count`) returns `(length, true)`.
        if lineStart < self.length {
            if li == lineIndex {
                return (lineStart, true)
            }
            if lineIndex == li + 1 {
                return (self.length, true)
            }
            return (self.length, false)
        }
        if li == lineIndex {
            (self.length, true)
        } else {
            (self.length, false)
        }
    }

    /// Internal: build a sub-view over byte range `[startByte, endByte)`.
    fileprivate func _subView(startByte startByte: Int64, endByte endByte: Int64) -> LinesView {
        LinesView(slice: self.slice.subslice(from: self.slice.start + startByte, to: self.slice.start + endByte))
    }
}

// ============================================================================
// STRING INDEX TYPES
// ============================================================================

/// A typed wrapper for a byte position within a `String`.
///
/// `ByteIndex` exists so that APIs taking string positions can refuse
/// raw `Int64`s, which removes the "is this a byte offset or a char
/// offset?" ambiguity at the call site. The wrapped `value` is a
/// plain UTF-8 byte offset; arithmetic is the caller's responsibility.
///
/// # Representation
///
/// A single `Int64` field.
public struct ByteIndex: Equatable, Comparable {
    /// The wrapped byte offset.
    public var value: Int64

    /// @name From Value
    /// Wraps a raw byte offset.
    public init(value: Int64) {
        self.value = value;
    }

    /// Returns true if the two indices wrap the same byte offset.
    public func isEqual(to other: ByteIndex) -> Bool {
        self.value == other.value
    }

    /// Compares two byte indices by their wrapped offsets.
    public func compare(other: ByteIndex) -> Ordering {
        self.value.compare(other.value)
    }
}

/// A typed wrapper for a character position within a `String`.
///
/// Unlike `ByteIndex`, `CharIndex` carries the byte offset of the
/// underlying character — code-point indexing is O(n), so this
/// pre-resolved offset is what gets stored. Construct one by walking
/// the string yourself; the type is purely a tag for clarity.
///
/// # Representation
///
/// A single `Int64` field holding the byte offset of the character.
public struct CharIndex: Equatable {
    /// The byte offset where the indexed character begins.
    public var byteOffset: Int64

    /// @name From Offset
    /// Wraps a pre-resolved byte offset for a character position.
    public init(byteOffset: Int64) {
        self.byteOffset = byteOffset;
    }

    /// Returns true if the two indices point at the same byte offset.
    public func isEqual(to other: CharIndex) -> Bool {
        self.byteOffset == other.byteOffset
    }
}

/// A typed wrapper for a grapheme-cluster position within a `String`.
///
/// Like `CharIndex` but ranges over UAX #29 clusters rather than
/// code points. Stores the byte offset of the cluster's first byte;
/// resolving requires walking the segmenter.
///
/// # Representation
///
/// A single `Int64` field holding the byte offset of the grapheme.
public struct GraphemeIndex: Equatable {
    /// The byte offset where the indexed grapheme begins.
    public var byteOffset: Int64

    /// @name From Offset
    /// Wraps a pre-resolved byte offset for a grapheme position.
    public init(byteOffset: Int64) {
        self.byteOffset = byteOffset;
    }

    /// Returns true if the two indices point at the same byte offset.
    public func isEqual(to other: GraphemeIndex) -> Bool {
        self.byteOffset == other.byteOffset
    }
}

// ============================================================================
// BYTES VIEW INDEX PROTOCOLS
// ============================================================================

/// Stdlib-internal index types for `BytesView` subscripts.
///
/// `Int64` reads a single byte (`UInt8`); range types yield a zero-copy
/// `BytesView` sub-view. All access is O(1) — no buffer copy on slice.
internal protocol BytesIndex {
    type BytesYield
    func readBytes(from view: BytesView) -> BytesYield
    func readBytesChecked(from view: BytesView) -> BytesYield?
    func readBytesUnchecked(from view: BytesView) -> BytesYield
}

internal protocol BytesClampable {
    type BytesClampedYield
    func readBytesClamped(from view: BytesView) -> BytesClampedYield
}

internal protocol BytesWrappable {
    type BytesWrappedYield
    func readBytesWrapped(from view: BytesView) -> BytesWrappedYield
}

extend Int64: BytesIndex {
    type BytesYield = UInt8

    public func readBytes(from view: BytesView) -> UInt8 {
        if self < 0 or self >= view.count {
            fatalError("BytesView index out of bounds")
        }
        view._readByteRaw(index: self)
    }

    public func readBytesChecked(from view: BytesView) -> UInt8? {
        if self >= 0 and self < view.count {
            .Some(view._readByteRaw(index: self))
        } else {
            .None
        }
    }

    public func readBytesUnchecked(from view: BytesView) -> UInt8 {
        view._readByteRaw(index: self)
    }
}

extend Int64: BytesClampable {
    type BytesClampedYield = UInt8?

    public func readBytesClamped(from view: BytesView) -> UInt8? {
        let len = view.count;
        if len == 0 {
            return .None
        }
        var idx = self;
        if idx < 0 { idx = 0 }
        if idx >= len { idx = len - 1 }
        .Some(view._readByteRaw(index: idx))
    }
}

extend Int64: BytesWrappable {
    type BytesWrappedYield = UInt8?

    public func readBytesWrapped(from view: BytesView) -> UInt8? {
        let len = view.count;
        if len == 0 {
            return .None
        }
        var idx = self % len;
        if idx < 0 { idx = idx + len }
        .Some(view._readByteRaw(index: idx))
    }
}

// Internal helper: copy bytes `[startByte, endByte)` from a raw UTF-8
// buffer into a fresh `String`. No validation; caller ensures sane
// bounds. Routes through `String.fromRawBytes` so the bulk copy lowers
// to libc `memcpy`.
fileprivate func _copyByteRange(ptr ptr: lang.ptr[lang.i8], startByte startByte: Int64, endByte endByte: Int64) -> String {
    let count = endByte - startByte;
    if count <= 0 {
        return String()
    }
    let srcAt: lang.ptr[lang.i8] = lang.ptr_offset[lang.i8](ptr, startByte.raw);
    String.fromRawBytes(srcAt, count)
}

extend Range[Int64]: BytesIndex {
    type BytesYield = BytesView

    public func readBytes(from view: BytesView) -> BytesView {
        if self.start < 0 or self.start > self.end or self.end > view.count {
            fatalError("BytesView range out of bounds")
        }
        view._subView(startByte: self.start, endByte: self.end)
    }

    public func readBytesChecked(from view: BytesView) -> BytesView? {
        if self.start < 0 or self.start > self.end or self.end > view.count {
            return .None
        }
        .Some(view._subView(startByte: self.start, endByte: self.end))
    }

    public func readBytesUnchecked(from view: BytesView) -> BytesView {
        view._subView(startByte: self.start, endByte: self.end)
    }
}

extend Range[Int64]: BytesClampable {
    type BytesClampedYield = BytesView

    public func readBytesClamped(from view: BytesView) -> BytesView {
        let len = view.count;
        var start = self.start;
        var end = self.end;
        if start < 0 { start = 0 }
        if end > len { end = len }
        if start > end { start = end }
        view._subView(startByte: start, endByte: end)
    }
}

extend Range[Int64]: BytesWrappable {
    type BytesWrappedYield = BytesView

    public func readBytesWrapped(from view: BytesView) -> BytesView {
        let len = view.count;
        if len == 0 {
            return view._subView(startByte: 0, endByte: 0)
        }
        var s = self.start % len;
        if s < 0 { s = s + len }
        var span = self.end - self.start;
        if span < 0 { span = 0 }
        var e = s + span;
        if e > len { e = len }
        view._subView(startByte: s, endByte: e)
    }
}

extend ClosedRange[Int64]: BytesIndex {
    type BytesYield = BytesView

    public func readBytes(from view: BytesView) -> BytesView {
        let endExclusive = self.end + 1;
        if self.start < 0 or self.start > endExclusive or endExclusive > view.count {
            fatalError("BytesView range out of bounds")
        }
        view._subView(startByte: self.start, endByte: endExclusive)
    }

    public func readBytesChecked(from view: BytesView) -> BytesView? {
        let endExclusive = self.end + 1;
        if self.start < 0 or self.start > endExclusive or endExclusive > view.count {
            return .None
        }
        .Some(view._subView(startByte: self.start, endByte: endExclusive))
    }

    public func readBytesUnchecked(from view: BytesView) -> BytesView {
        let endExclusive = self.end + 1;
        view._subView(startByte: self.start, endByte: endExclusive)
    }
}

extend ClosedRange[Int64]: BytesClampable {
    type BytesClampedYield = BytesView

    public func readBytesClamped(from view: BytesView) -> BytesView {
        let len = view.count;
        var start = self.start;
        var end = self.end + 1;
        if start < 0 { start = 0 }
        if end > len { end = len }
        if start > end { start = end }
        view._subView(startByte: start, endByte: end)
    }
}

extend ClosedRange[Int64]: BytesWrappable {
    type BytesWrappedYield = BytesView

    public func readBytesWrapped(from view: BytesView) -> BytesView {
        let len = view.count;
        if len == 0 {
            return view._subView(startByte: 0, endByte: 0)
        }
        var s = self.start % len;
        if s < 0 { s = s + len }
        var span = self.end - self.start + 1;
        if span < 0 { span = 0 }
        var e = s + span;
        if e > len { e = len }
        view._subView(startByte: s, endByte: e)
    }
}

/// Range-only index for `BytesView.substring`. Conformed by every
/// range type so a single generic `substring` can dispatch over all of
/// them. Single-element indexes (`Int64`) deliberately don't conform —
/// `substring` is range-flavored only.
public protocol BytesSubstringIndex {
    func readBytesSubstring(from view: BytesView) -> String
}

extend Range[Int64]: BytesSubstringIndex {
    public func readBytesSubstring(from view: BytesView) -> String {
        view(self).toString()
    }
}

extend ClosedRange[Int64]: BytesSubstringIndex {
    public func readBytesSubstring(from view: BytesView) -> String {
        view(self).toString()
    }
}

extend RangeFrom[Int64]: BytesIndex {
    type BytesYield = BytesView

    public func readBytes(from view: BytesView) -> BytesView {
        let start = self.start;
        let len = view.count;
        if start < 0 or start > len {
            fatalError("BytesView range out of bounds")
        }
        view._subView(startByte: start, endByte: len)
    }

    public func readBytesChecked(from view: BytesView) -> BytesView? {
        let start = self.start;
        let len = view.count;
        if start < 0 or start > len {
            return .None
        }
        .Some(view._subView(startByte: start, endByte: len))
    }

    public func readBytesUnchecked(from view: BytesView) -> BytesView {
        view._subView(startByte: self.start, endByte: view.count)
    }
}

extend RangeFrom[Int64]: BytesClampable {
    type BytesClampedYield = BytesView

    public func readBytesClamped(from view: BytesView) -> BytesView {
        let len = view.count;
        var start = self.start;
        if start < 0 { start = 0 }
        if start > len { start = len }
        view._subView(startByte: start, endByte: len)
    }
}

extend RangeFrom[Int64]: BytesWrappable {
    type BytesWrappedYield = BytesView

    public func readBytesWrapped(from view: BytesView) -> BytesView {
        let len = view.count;
        if len == 0 {
            return view._subView(startByte: 0, endByte: 0)
        }
        var s = self.start % len;
        if s < 0 { s = s + len }
        view._subView(startByte: s, endByte: len)
    }
}

extend RangeFrom[Int64]: BytesSubstringIndex {
    public func readBytesSubstring(from view: BytesView) -> String {
        view(self).toString()
    }
}

extend RangeUpTo[Int64]: BytesIndex {
    type BytesYield = BytesView

    public func readBytes(from view: BytesView) -> BytesView {
        let end = self.end;
        if end < 0 or end > view.count {
            fatalError("BytesView range out of bounds")
        }
        view._subView(startByte: 0, endByte: end)
    }

    public func readBytesChecked(from view: BytesView) -> BytesView? {
        let end = self.end;
        if end < 0 or end > view.count {
            return .None
        }
        .Some(view._subView(startByte: 0, endByte: end))
    }

    public func readBytesUnchecked(from view: BytesView) -> BytesView {
        view._subView(startByte: 0, endByte: self.end)
    }
}

extend RangeUpTo[Int64]: BytesClampable {
    type BytesClampedYield = BytesView

    public func readBytesClamped(from view: BytesView) -> BytesView {
        let len = view.count;
        var end = self.end;
        if end < 0 { end = 0 }
        if end > len { end = len }
        view._subView(startByte: 0, endByte: end)
    }
}

extend RangeUpTo[Int64]: BytesWrappable {
    type BytesWrappedYield = BytesView

    public func readBytesWrapped(from view: BytesView) -> BytesView {
        let len = view.count;
        if len == 0 {
            return view._subView(startByte: 0, endByte: 0)
        }
        var e = self.end % len;
        if e < 0 { e = e + len }
        view._subView(startByte: 0, endByte: e)
    }
}

extend RangeUpTo[Int64]: BytesSubstringIndex {
    public func readBytesSubstring(from view: BytesView) -> String {
        view(self).toString()
    }
}

extend RangeThrough[Int64]: BytesIndex {
    type BytesYield = BytesView

    public func readBytes(from view: BytesView) -> BytesView {
        let endExclusive = self.end + 1;
        if endExclusive < 0 or endExclusive > view.count {
            fatalError("BytesView range out of bounds")
        }
        view._subView(startByte: 0, endByte: endExclusive)
    }

    public func readBytesChecked(from view: BytesView) -> BytesView? {
        let endExclusive = self.end + 1;
        if endExclusive < 0 or endExclusive > view.count {
            return .None
        }
        .Some(view._subView(startByte: 0, endByte: endExclusive))
    }

    public func readBytesUnchecked(from view: BytesView) -> BytesView {
        view._subView(startByte: 0, endByte: self.end + 1)
    }
}

extend RangeThrough[Int64]: BytesClampable {
    type BytesClampedYield = BytesView

    public func readBytesClamped(from view: BytesView) -> BytesView {
        let len = view.count;
        var end = self.end + 1;
        if end < 0 { end = 0 }
        if end > len { end = len }
        view._subView(startByte: 0, endByte: end)
    }
}

extend RangeThrough[Int64]: BytesWrappable {
    type BytesWrappedYield = BytesView

    public func readBytesWrapped(from view: BytesView) -> BytesView {
        let len = view.count;
        if len == 0 {
            return view._subView(startByte: 0, endByte: 0)
        }
        var e = self.end % len;
        if e < 0 { e = e + len }
        e = e + 1;
        view._subView(startByte: 0, endByte: e)
    }
}

extend RangeThrough[Int64]: BytesSubstringIndex {
    public func readBytesSubstring(from view: BytesView) -> String {
        view(self).toString()
    }
}

// ============================================================================
// CHARS VIEW INDEX PROTOCOLS
// ============================================================================

/// Stdlib-internal index types for `CharsView` subscripts.
///
/// `Int64` reads a single code point (`Char`); range types yield a
/// zero-copy `CharsView` sub-view. All access is O(n) — UTF-8 is
/// variable-width, so every char-index lookup walks the buffer; the
/// slice itself is free (no copy), but resolving the byte offsets is
/// linear.
internal protocol CharsIndex {
    type CharsYield
    func readChars(from view: CharsView) -> CharsYield
    func readCharsChecked(from view: CharsView) -> CharsYield?
}

internal protocol CharsClampable {
    type CharsClampedYield
    func readCharsClamped(from view: CharsView) -> CharsClampedYield
}

internal protocol CharsWrappable {
    type CharsWrappedYield
    func readCharsWrapped(from view: CharsView) -> CharsWrappedYield
}

// Internal: walk to char-index `i` and decode that code point, or
// return `.None` if `i` is past the end.
fileprivate func _charsViewCharAt(view view: CharsView, charIndex charIndex: Int64) -> Char? {
    if charIndex < 0 {
        return .None
    }
    var ci: Int64 = 0;
    var it = view.iter();
    while true {
        let next = it.next();
        if let .Some(c) = next {
            if ci == charIndex {
                return .Some(c)
            }
            ci = ci + 1
        } else {
            return .None
        }
    }
    .None
}

extend Int64: CharsIndex {
    type CharsYield = Char

    public func readChars(from view: CharsView) -> Char {
        match _charsViewCharAt(view: view, charIndex: self) {
            .Some(c) => c,
            .None => fatalError("CharsView index out of bounds")
        }
    }

    public func readCharsChecked(from view: CharsView) -> Char? {
        _charsViewCharAt(view: view, charIndex: self)
    }
}

extend Int64: CharsClampable {
    type CharsClampedYield = Char?

    public func readCharsClamped(from view: CharsView) -> Char? {
        let n = view.count;
        if n == 0 {
            return .None
        }
        var idx = self;
        if idx < 0 { idx = 0 }
        if idx >= n { idx = n - 1 }
        _charsViewCharAt(view: view, charIndex: idx)
    }
}

extend Int64: CharsWrappable {
    type CharsWrappedYield = Char?

    public func readCharsWrapped(from view: CharsView) -> Char? {
        let n = view.count;
        if n == 0 {
            return .None
        }
        var idx = self % n;
        if idx < 0 { idx = idx + n }
        _charsViewCharAt(view: view, charIndex: idx)
    }
}

extend Range[Int64]: CharsIndex {
    type CharsYield = CharsView

    public func readChars(from view: CharsView) -> CharsView {
        let s = self.start;
        let e = self.end;
        if s < 0 or s > e {
            fatalError("CharsView range out of bounds")
        }
        let (startByte, foundStart) = view._byteOffsetForCharIndex(charIndex: s);
        let (endByte, foundEnd) = view._byteOffsetForCharIndex(charIndex: e);
        if foundStart == false or foundEnd == false {
            fatalError("CharsView range out of bounds")
        }
        view._subView(startByte: startByte, endByte: endByte)
    }

    public func readCharsChecked(from view: CharsView) -> CharsView? {
        let s = self.start;
        let e = self.end;
        if s < 0 or s > e {
            return .None
        }
        let (startByte, foundStart) = view._byteOffsetForCharIndex(charIndex: s);
        let (endByte, foundEnd) = view._byteOffsetForCharIndex(charIndex: e);
        if foundStart == false or foundEnd == false {
            return .None
        }
        .Some(view._subView(startByte: startByte, endByte: endByte))
    }
}

extend Range[Int64]: CharsClampable {
    type CharsClampedYield = CharsView

    public func readCharsClamped(from view: CharsView) -> CharsView {
        let n = view.count;
        var s = self.start;
        var e = self.end;
        if s < 0 { s = 0 }
        if e > n { e = n }
        if s > e { s = e }
        let (startByte, _) = view._byteOffsetForCharIndex(charIndex: s);
        let (endByte, _) = view._byteOffsetForCharIndex(charIndex: e);
        view._subView(startByte: startByte, endByte: endByte)
    }
}

extend Range[Int64]: CharsWrappable {
    type CharsWrappedYield = CharsView

    public func readCharsWrapped(from view: CharsView) -> CharsView {
        let n = view.count;
        if n == 0 {
            return view._subView(startByte: 0, endByte: 0)
        }
        var s = self.start % n;
        if s < 0 { s = s + n }
        var span = self.end - self.start;
        if span < 0 { span = 0 }
        var e = s + span;
        if e > n { e = n }
        let (startByte, _) = view._byteOffsetForCharIndex(charIndex: s);
        let (endByte, _) = view._byteOffsetForCharIndex(charIndex: e);
        view._subView(startByte: startByte, endByte: endByte)
    }
}

extend ClosedRange[Int64]: CharsIndex {
    type CharsYield = CharsView

    public func readChars(from view: CharsView) -> CharsView {
        let s = self.start;
        let endInclusive = self.end;
        let endExclusive = endInclusive + 1;
        if s < 0 or s > endExclusive {
            fatalError("CharsView range out of bounds")
        }
        let (startByte, foundStart) = view._byteOffsetForCharIndex(charIndex: s);
        let (endByte, foundEnd) = view._byteOffsetForCharIndex(charIndex: endExclusive);
        if foundStart == false or foundEnd == false {
            fatalError("CharsView range out of bounds")
        }
        view._subView(startByte: startByte, endByte: endByte)
    }

    public func readCharsChecked(from view: CharsView) -> CharsView? {
        let s = self.start;
        let endExclusive = self.end + 1;
        if s < 0 or s > endExclusive {
            return .None
        }
        let (startByte, foundStart) = view._byteOffsetForCharIndex(charIndex: s);
        let (endByte, foundEnd) = view._byteOffsetForCharIndex(charIndex: endExclusive);
        if foundStart == false or foundEnd == false {
            return .None
        }
        .Some(view._subView(startByte: startByte, endByte: endByte))
    }
}

extend ClosedRange[Int64]: CharsClampable {
    type CharsClampedYield = CharsView

    public func readCharsClamped(from view: CharsView) -> CharsView {
        let n = view.count;
        var s = self.start;
        var e = self.end + 1;
        if s < 0 { s = 0 }
        if e > n { e = n }
        if s > e { s = e }
        let (startByte, _) = view._byteOffsetForCharIndex(charIndex: s);
        let (endByte, _) = view._byteOffsetForCharIndex(charIndex: e);
        view._subView(startByte: startByte, endByte: endByte)
    }
}

extend ClosedRange[Int64]: CharsWrappable {
    type CharsWrappedYield = CharsView

    public func readCharsWrapped(from view: CharsView) -> CharsView {
        let n = view.count;
        if n == 0 {
            return view._subView(startByte: 0, endByte: 0)
        }
        var s = self.start % n;
        if s < 0 { s = s + n }
        var span = self.end - self.start + 1;
        if span < 0 { span = 0 }
        var e = s + span;
        if e > n { e = n }
        let (startByte, _) = view._byteOffsetForCharIndex(charIndex: s);
        let (endByte, _) = view._byteOffsetForCharIndex(charIndex: e);
        view._subView(startByte: startByte, endByte: endByte)
    }
}

/// Range-only index for `CharsView.substring`. See `BytesSubstringIndex`.
public protocol CharsSubstringIndex {
    func readCharsSubstring(from view: CharsView) -> String
}

extend Range[Int64]: CharsSubstringIndex {
    public func readCharsSubstring(from view: CharsView) -> String {
        view(self).toString()
    }
}

extend ClosedRange[Int64]: CharsSubstringIndex {
    public func readCharsSubstring(from view: CharsView) -> String {
        view(self).toString()
    }
}

extend RangeFrom[Int64]: CharsIndex {
    type CharsYield = CharsView

    public func readChars(from view: CharsView) -> CharsView {
        let s = self.start;
        let n = view.count;
        if s < 0 or s > n {
            fatalError("CharsView range out of bounds")
        }
        let (startByte, foundStart) = view._byteOffsetForCharIndex(charIndex: s);
        let (endByte, foundEnd) = view._byteOffsetForCharIndex(charIndex: n);
        if foundStart == false or foundEnd == false {
            fatalError("CharsView range out of bounds")
        }
        view._subView(startByte: startByte, endByte: endByte)
    }

    public func readCharsChecked(from view: CharsView) -> CharsView? {
        let s = self.start;
        let n = view.count;
        if s < 0 or s > n {
            return .None
        }
        let (startByte, foundStart) = view._byteOffsetForCharIndex(charIndex: s);
        let (endByte, foundEnd) = view._byteOffsetForCharIndex(charIndex: n);
        if foundStart == false or foundEnd == false {
            return .None
        }
        .Some(view._subView(startByte: startByte, endByte: endByte))
    }
}

extend RangeFrom[Int64]: CharsClampable {
    type CharsClampedYield = CharsView

    public func readCharsClamped(from view: CharsView) -> CharsView {
        let n = view.count;
        var s = self.start;
        if s < 0 { s = 0 }
        if s > n { s = n }
        let (startByte, _) = view._byteOffsetForCharIndex(charIndex: s);
        let (endByte, _) = view._byteOffsetForCharIndex(charIndex: n);
        view._subView(startByte: startByte, endByte: endByte)
    }
}

extend RangeFrom[Int64]: CharsWrappable {
    type CharsWrappedYield = CharsView

    public func readCharsWrapped(from view: CharsView) -> CharsView {
        let n = view.count;
        if n == 0 {
            return view._subView(startByte: 0, endByte: 0)
        }
        var s = self.start % n;
        if s < 0 { s = s + n }
        let (startByte, _) = view._byteOffsetForCharIndex(charIndex: s);
        let (endByte, _) = view._byteOffsetForCharIndex(charIndex: n);
        view._subView(startByte: startByte, endByte: endByte)
    }
}

extend RangeFrom[Int64]: CharsSubstringIndex {
    public func readCharsSubstring(from view: CharsView) -> String {
        view(self).toString()
    }
}

extend RangeUpTo[Int64]: CharsIndex {
    type CharsYield = CharsView

    public func readChars(from view: CharsView) -> CharsView {
        let e = self.end;
        if e < 0 or e > view.count {
            fatalError("CharsView range out of bounds")
        }
        let (endByte, foundEnd) = view._byteOffsetForCharIndex(charIndex: e);
        if foundEnd == false {
            fatalError("CharsView range out of bounds")
        }
        view._subView(startByte: 0, endByte: endByte)
    }

    public func readCharsChecked(from view: CharsView) -> CharsView? {
        let e = self.end;
        if e < 0 or e > view.count {
            return .None
        }
        let (endByte, foundEnd) = view._byteOffsetForCharIndex(charIndex: e);
        if foundEnd == false {
            return .None
        }
        .Some(view._subView(startByte: 0, endByte: endByte))
    }
}

extend RangeUpTo[Int64]: CharsClampable {
    type CharsClampedYield = CharsView

    public func readCharsClamped(from view: CharsView) -> CharsView {
        let n = view.count;
        var e = self.end;
        if e < 0 { e = 0 }
        if e > n { e = n }
        let (endByte, _) = view._byteOffsetForCharIndex(charIndex: e);
        view._subView(startByte: 0, endByte: endByte)
    }
}

extend RangeUpTo[Int64]: CharsWrappable {
    type CharsWrappedYield = CharsView

    public func readCharsWrapped(from view: CharsView) -> CharsView {
        let n = view.count;
        if n == 0 {
            return view._subView(startByte: 0, endByte: 0)
        }
        var e = self.end % n;
        if e < 0 { e = e + n }
        let (endByte, _) = view._byteOffsetForCharIndex(charIndex: e);
        view._subView(startByte: 0, endByte: endByte)
    }
}

extend RangeUpTo[Int64]: CharsSubstringIndex {
    public func readCharsSubstring(from view: CharsView) -> String {
        view(self).toString()
    }
}

extend RangeThrough[Int64]: CharsIndex {
    type CharsYield = CharsView

    public func readChars(from view: CharsView) -> CharsView {
        let endExclusive = self.end + 1;
        if endExclusive < 0 or endExclusive > view.count {
            fatalError("CharsView range out of bounds")
        }
        let (endByte, foundEnd) = view._byteOffsetForCharIndex(charIndex: endExclusive);
        if foundEnd == false {
            fatalError("CharsView range out of bounds")
        }
        view._subView(startByte: 0, endByte: endByte)
    }

    public func readCharsChecked(from view: CharsView) -> CharsView? {
        let endExclusive = self.end + 1;
        if endExclusive < 0 or endExclusive > view.count {
            return .None
        }
        let (endByte, foundEnd) = view._byteOffsetForCharIndex(charIndex: endExclusive);
        if foundEnd == false {
            return .None
        }
        .Some(view._subView(startByte: 0, endByte: endByte))
    }
}

extend RangeThrough[Int64]: CharsClampable {
    type CharsClampedYield = CharsView

    public func readCharsClamped(from view: CharsView) -> CharsView {
        let n = view.count;
        var e = self.end + 1;
        if e < 0 { e = 0 }
        if e > n { e = n }
        let (endByte, _) = view._byteOffsetForCharIndex(charIndex: e);
        view._subView(startByte: 0, endByte: endByte)
    }
}

extend RangeThrough[Int64]: CharsWrappable {
    type CharsWrappedYield = CharsView

    public func readCharsWrapped(from view: CharsView) -> CharsView {
        let n = view.count;
        if n == 0 {
            return view._subView(startByte: 0, endByte: 0)
        }
        var e = self.end % n;
        if e < 0 { e = e + n }
        e = e + 1;
        let (endByte, _) = view._byteOffsetForCharIndex(charIndex: e);
        view._subView(startByte: 0, endByte: endByte)
    }
}

extend RangeThrough[Int64]: CharsSubstringIndex {
    public func readCharsSubstring(from view: CharsView) -> String {
        view(self).toString()
    }
}

// ============================================================================
// GRAPHEMES VIEW INDEX PROTOCOLS
// ============================================================================

/// Stdlib-internal index types for `GraphemesView` subscripts.
///
/// `Int64` reads a single cluster (`Grapheme`); range types yield a
/// zero-copy `GraphemesView` sub-view. All access is O(n) — every
/// cluster boundary is found by walking the UAX #29 segmenter from the
/// start; the slice itself is free.
internal protocol GraphemesIndex {
    type GraphemesYield
    func readGraphemes(from view: GraphemesView) -> GraphemesYield
    func readGraphemesChecked(from view: GraphemesView) -> GraphemesYield?
}

internal protocol GraphemesClampable {
    type GraphemesClampedYield
    func readGraphemesClamped(from view: GraphemesView) -> GraphemesClampedYield
}

internal protocol GraphemesWrappable {
    type GraphemesWrappedYield
    func readGraphemesWrapped(from view: GraphemesView) -> GraphemesWrappedYield
}

// Internal: walk the segmenter to grapheme index `i` and return that
// cluster, or `.None` if `i` is past the end (or negative).
fileprivate func _graphemesViewAt(view view: GraphemesView, graphemeIndex graphemeIndex: Int64) -> Grapheme? {
    if graphemeIndex < 0 {
        return .None
    }
    var gi: Int64 = 0;
    var it = view.iter();
    while true {
        let next = it.next();
        if let .Some(g) = next {
            if gi == graphemeIndex {
                return .Some(g)
            }
            gi = gi + 1
        } else {
            return .None
        }
    }
    .None
}

extend Int64: GraphemesIndex {
    type GraphemesYield = Grapheme

    public func readGraphemes(from view: GraphemesView) -> Grapheme {
        match _graphemesViewAt(view: view, graphemeIndex: self) {
            .Some(g) => g,
            .None => fatalError("GraphemesView index out of bounds")
        }
    }

    public func readGraphemesChecked(from view: GraphemesView) -> Grapheme? {
        _graphemesViewAt(view: view, graphemeIndex: self)
    }
}

extend Int64: GraphemesClampable {
    type GraphemesClampedYield = Grapheme?

    public func readGraphemesClamped(from view: GraphemesView) -> Grapheme? {
        let n = view.count;
        if n == 0 {
            return .None
        }
        var idx = self;
        if idx < 0 { idx = 0 }
        if idx >= n { idx = n - 1 }
        _graphemesViewAt(view: view, graphemeIndex: idx)
    }
}

extend Int64: GraphemesWrappable {
    type GraphemesWrappedYield = Grapheme?

    public func readGraphemesWrapped(from view: GraphemesView) -> Grapheme? {
        let n = view.count;
        if n == 0 {
            return .None
        }
        var idx = self % n;
        if idx < 0 { idx = idx + n }
        _graphemesViewAt(view: view, graphemeIndex: idx)
    }
}

extend Range[Int64]: GraphemesIndex {
    type GraphemesYield = GraphemesView

    public func readGraphemes(from view: GraphemesView) -> GraphemesView {
        let s = self.start;
        let e = self.end;
        if s < 0 or s > e {
            fatalError("GraphemesView range out of bounds")
        }
        let (startByte, foundStart) = view._byteOffsetForGraphemeIndex(graphemeIndex: s);
        let (endByte, foundEnd) = view._byteOffsetForGraphemeIndex(graphemeIndex: e);
        if foundStart == false or foundEnd == false {
            fatalError("GraphemesView range out of bounds")
        }
        view._subView(startByte: startByte, endByte: endByte)
    }

    public func readGraphemesChecked(from view: GraphemesView) -> GraphemesView? {
        let s = self.start;
        let e = self.end;
        if s < 0 or s > e {
            return .None
        }
        let (startByte, foundStart) = view._byteOffsetForGraphemeIndex(graphemeIndex: s);
        let (endByte, foundEnd) = view._byteOffsetForGraphemeIndex(graphemeIndex: e);
        if foundStart == false or foundEnd == false {
            return .None
        }
        .Some(view._subView(startByte: startByte, endByte: endByte))
    }
}

extend Range[Int64]: GraphemesClampable {
    type GraphemesClampedYield = GraphemesView

    public func readGraphemesClamped(from view: GraphemesView) -> GraphemesView {
        let n = view.count;
        var s = self.start;
        var e = self.end;
        if s < 0 { s = 0 }
        if e > n { e = n }
        if s > e { s = e }
        let (startByte, _) = view._byteOffsetForGraphemeIndex(graphemeIndex: s);
        let (endByte, _) = view._byteOffsetForGraphemeIndex(graphemeIndex: e);
        view._subView(startByte: startByte, endByte: endByte)
    }
}

extend Range[Int64]: GraphemesWrappable {
    type GraphemesWrappedYield = GraphemesView

    public func readGraphemesWrapped(from view: GraphemesView) -> GraphemesView {
        let n = view.count;
        if n == 0 {
            return view._subView(startByte: 0, endByte: 0)
        }
        var s = self.start % n;
        if s < 0 { s = s + n }
        var span = self.end - self.start;
        if span < 0 { span = 0 }
        var e = s + span;
        if e > n { e = n }
        let (startByte, _) = view._byteOffsetForGraphemeIndex(graphemeIndex: s);
        let (endByte, _) = view._byteOffsetForGraphemeIndex(graphemeIndex: e);
        view._subView(startByte: startByte, endByte: endByte)
    }
}

extend ClosedRange[Int64]: GraphemesIndex {
    type GraphemesYield = GraphemesView

    public func readGraphemes(from view: GraphemesView) -> GraphemesView {
        let s = self.start;
        let endExclusive = self.end + 1;
        if s < 0 or s > endExclusive {
            fatalError("GraphemesView range out of bounds")
        }
        let (startByte, foundStart) = view._byteOffsetForGraphemeIndex(graphemeIndex: s);
        let (endByte, foundEnd) = view._byteOffsetForGraphemeIndex(graphemeIndex: endExclusive);
        if foundStart == false or foundEnd == false {
            fatalError("GraphemesView range out of bounds")
        }
        view._subView(startByte: startByte, endByte: endByte)
    }

    public func readGraphemesChecked(from view: GraphemesView) -> GraphemesView? {
        let s = self.start;
        let endExclusive = self.end + 1;
        if s < 0 or s > endExclusive {
            return .None
        }
        let (startByte, foundStart) = view._byteOffsetForGraphemeIndex(graphemeIndex: s);
        let (endByte, foundEnd) = view._byteOffsetForGraphemeIndex(graphemeIndex: endExclusive);
        if foundStart == false or foundEnd == false {
            return .None
        }
        .Some(view._subView(startByte: startByte, endByte: endByte))
    }
}

extend ClosedRange[Int64]: GraphemesClampable {
    type GraphemesClampedYield = GraphemesView

    public func readGraphemesClamped(from view: GraphemesView) -> GraphemesView {
        let n = view.count;
        var s = self.start;
        var e = self.end + 1;
        if s < 0 { s = 0 }
        if e > n { e = n }
        if s > e { s = e }
        let (startByte, _) = view._byteOffsetForGraphemeIndex(graphemeIndex: s);
        let (endByte, _) = view._byteOffsetForGraphemeIndex(graphemeIndex: e);
        view._subView(startByte: startByte, endByte: endByte)
    }
}

extend ClosedRange[Int64]: GraphemesWrappable {
    type GraphemesWrappedYield = GraphemesView

    public func readGraphemesWrapped(from view: GraphemesView) -> GraphemesView {
        let n = view.count;
        if n == 0 {
            return view._subView(startByte: 0, endByte: 0)
        }
        var s = self.start % n;
        if s < 0 { s = s + n }
        var span = self.end - self.start + 1;
        if span < 0 { span = 0 }
        var e = s + span;
        if e > n { e = n }
        let (startByte, _) = view._byteOffsetForGraphemeIndex(graphemeIndex: s);
        let (endByte, _) = view._byteOffsetForGraphemeIndex(graphemeIndex: e);
        view._subView(startByte: startByte, endByte: endByte)
    }
}

/// Range-only index for `GraphemesView.substring`. See `BytesSubstringIndex`.
public protocol GraphemesSubstringIndex {
    func readGraphemesSubstring(from view: GraphemesView) -> String
}

extend Range[Int64]: GraphemesSubstringIndex {
    public func readGraphemesSubstring(from view: GraphemesView) -> String {
        view(self).toString()
    }
}

extend ClosedRange[Int64]: GraphemesSubstringIndex {
    public func readGraphemesSubstring(from view: GraphemesView) -> String {
        view(self).toString()
    }
}

extend RangeFrom[Int64]: GraphemesIndex {
    type GraphemesYield = GraphemesView

    public func readGraphemes(from view: GraphemesView) -> GraphemesView {
        let s = self.start;
        let n = view.count;
        if s < 0 or s > n {
            fatalError("GraphemesView range out of bounds")
        }
        let (startByte, foundStart) = view._byteOffsetForGraphemeIndex(graphemeIndex: s);
        let (endByte, foundEnd) = view._byteOffsetForGraphemeIndex(graphemeIndex: n);
        if foundStart == false or foundEnd == false {
            fatalError("GraphemesView range out of bounds")
        }
        view._subView(startByte: startByte, endByte: endByte)
    }

    public func readGraphemesChecked(from view: GraphemesView) -> GraphemesView? {
        let s = self.start;
        let n = view.count;
        if s < 0 or s > n {
            return .None
        }
        let (startByte, foundStart) = view._byteOffsetForGraphemeIndex(graphemeIndex: s);
        let (endByte, foundEnd) = view._byteOffsetForGraphemeIndex(graphemeIndex: n);
        if foundStart == false or foundEnd == false {
            return .None
        }
        .Some(view._subView(startByte: startByte, endByte: endByte))
    }
}

extend RangeFrom[Int64]: GraphemesClampable {
    type GraphemesClampedYield = GraphemesView

    public func readGraphemesClamped(from view: GraphemesView) -> GraphemesView {
        let n = view.count;
        var s = self.start;
        if s < 0 { s = 0 }
        if s > n { s = n }
        let (startByte, _) = view._byteOffsetForGraphemeIndex(graphemeIndex: s);
        let (endByte, _) = view._byteOffsetForGraphemeIndex(graphemeIndex: n);
        view._subView(startByte: startByte, endByte: endByte)
    }
}

extend RangeFrom[Int64]: GraphemesWrappable {
    type GraphemesWrappedYield = GraphemesView

    public func readGraphemesWrapped(from view: GraphemesView) -> GraphemesView {
        let n = view.count;
        if n == 0 {
            return view._subView(startByte: 0, endByte: 0)
        }
        var s = self.start % n;
        if s < 0 { s = s + n }
        let (startByte, _) = view._byteOffsetForGraphemeIndex(graphemeIndex: s);
        let (endByte, _) = view._byteOffsetForGraphemeIndex(graphemeIndex: n);
        view._subView(startByte: startByte, endByte: endByte)
    }
}

extend RangeFrom[Int64]: GraphemesSubstringIndex {
    public func readGraphemesSubstring(from view: GraphemesView) -> String {
        view(self).toString()
    }
}

extend RangeUpTo[Int64]: GraphemesIndex {
    type GraphemesYield = GraphemesView

    public func readGraphemes(from view: GraphemesView) -> GraphemesView {
        let e = self.end;
        if e < 0 or e > view.count {
            fatalError("GraphemesView range out of bounds")
        }
        let (endByte, foundEnd) = view._byteOffsetForGraphemeIndex(graphemeIndex: e);
        if foundEnd == false {
            fatalError("GraphemesView range out of bounds")
        }
        view._subView(startByte: 0, endByte: endByte)
    }

    public func readGraphemesChecked(from view: GraphemesView) -> GraphemesView? {
        let e = self.end;
        if e < 0 or e > view.count {
            return .None
        }
        let (endByte, foundEnd) = view._byteOffsetForGraphemeIndex(graphemeIndex: e);
        if foundEnd == false {
            return .None
        }
        .Some(view._subView(startByte: 0, endByte: endByte))
    }
}

extend RangeUpTo[Int64]: GraphemesClampable {
    type GraphemesClampedYield = GraphemesView

    public func readGraphemesClamped(from view: GraphemesView) -> GraphemesView {
        let n = view.count;
        var e = self.end;
        if e < 0 { e = 0 }
        if e > n { e = n }
        let (endByte, _) = view._byteOffsetForGraphemeIndex(graphemeIndex: e);
        view._subView(startByte: 0, endByte: endByte)
    }
}

extend RangeUpTo[Int64]: GraphemesWrappable {
    type GraphemesWrappedYield = GraphemesView

    public func readGraphemesWrapped(from view: GraphemesView) -> GraphemesView {
        let n = view.count;
        if n == 0 {
            return view._subView(startByte: 0, endByte: 0)
        }
        var e = self.end % n;
        if e < 0 { e = e + n }
        let (endByte, _) = view._byteOffsetForGraphemeIndex(graphemeIndex: e);
        view._subView(startByte: 0, endByte: endByte)
    }
}

extend RangeUpTo[Int64]: GraphemesSubstringIndex {
    public func readGraphemesSubstring(from view: GraphemesView) -> String {
        view(self).toString()
    }
}

extend RangeThrough[Int64]: GraphemesIndex {
    type GraphemesYield = GraphemesView

    public func readGraphemes(from view: GraphemesView) -> GraphemesView {
        let endExclusive = self.end + 1;
        if endExclusive < 0 or endExclusive > view.count {
            fatalError("GraphemesView range out of bounds")
        }
        let (endByte, foundEnd) = view._byteOffsetForGraphemeIndex(graphemeIndex: endExclusive);
        if foundEnd == false {
            fatalError("GraphemesView range out of bounds")
        }
        view._subView(startByte: 0, endByte: endByte)
    }

    public func readGraphemesChecked(from view: GraphemesView) -> GraphemesView? {
        let endExclusive = self.end + 1;
        if endExclusive < 0 or endExclusive > view.count {
            return .None
        }
        let (endByte, foundEnd) = view._byteOffsetForGraphemeIndex(graphemeIndex: endExclusive);
        if foundEnd == false {
            return .None
        }
        .Some(view._subView(startByte: 0, endByte: endByte))
    }
}

extend RangeThrough[Int64]: GraphemesClampable {
    type GraphemesClampedYield = GraphemesView

    public func readGraphemesClamped(from view: GraphemesView) -> GraphemesView {
        let n = view.count;
        var e = self.end + 1;
        if e < 0 { e = 0 }
        if e > n { e = n }
        let (endByte, _) = view._byteOffsetForGraphemeIndex(graphemeIndex: e);
        view._subView(startByte: 0, endByte: endByte)
    }
}

extend RangeThrough[Int64]: GraphemesWrappable {
    type GraphemesWrappedYield = GraphemesView

    public func readGraphemesWrapped(from view: GraphemesView) -> GraphemesView {
        let n = view.count;
        if n == 0 {
            return view._subView(startByte: 0, endByte: 0)
        }
        var e = self.end % n;
        if e < 0 { e = e + n }
        e = e + 1;
        let (endByte, _) = view._byteOffsetForGraphemeIndex(graphemeIndex: e);
        view._subView(startByte: 0, endByte: endByte)
    }
}

extend RangeThrough[Int64]: GraphemesSubstringIndex {
    public func readGraphemesSubstring(from view: GraphemesView) -> String {
        view(self).toString()
    }
}

// ============================================================================
// LINES VIEW INDEX PROTOCOLS
// ============================================================================

/// Stdlib-internal index types for `LinesView` subscripts.
///
/// `Int64` reads a single line as a `String` (without terminator).
/// `Range[Int64]` / `ClosedRange[Int64]` read a contiguous run of lines
/// as a `LinesView` sub-view — the underlying byte range still includes
/// the original terminators, so iterating the sub-view round-trips the
/// same line strings.
internal protocol LinesIndex {
    type LinesYield
    func readLines(from view: LinesView) -> LinesYield
    func readLinesChecked(from view: LinesView) -> LinesYield?
}

internal protocol LinesClampable {
    type LinesClampedYield
    func readLinesClamped(from view: LinesView) -> LinesClampedYield
}

internal protocol LinesWrappable {
    type LinesWrappedYield
    func readLinesWrapped(from view: LinesView) -> LinesWrappedYield
}

// Internal: walk the iterator to line index `i` and return that line,
// or `.None` if `i` is past the end (or negative).
fileprivate func _linesViewAt(view view: LinesView, lineIndex lineIndex: Int64) -> String? {
    if lineIndex < 0 {
        return .None
    }
    var li: Int64 = 0;
    var it = view.iter();
    while true {
        let next = it.next();
        if let .Some(line) = next {
            if li == lineIndex {
                return .Some(line)
            }
            li = li + 1
        } else {
            return .None
        }
    }
    .None
}

extend Int64: LinesIndex {
    type LinesYield = String

    public func readLines(from view: LinesView) -> String {
        match _linesViewAt(view: view, lineIndex: self) {
            .Some(s) => s,
            .None => fatalError("LinesView index out of bounds")
        }
    }

    public func readLinesChecked(from view: LinesView) -> String? {
        _linesViewAt(view: view, lineIndex: self)
    }
}

extend Int64: LinesClampable {
    type LinesClampedYield = String?

    public func readLinesClamped(from view: LinesView) -> String? {
        let n = view.count;
        if n == 0 {
            return .None
        }
        var idx = self;
        if idx < 0 { idx = 0 }
        if idx >= n { idx = n - 1 }
        _linesViewAt(view: view, lineIndex: idx)
    }
}

extend Int64: LinesWrappable {
    type LinesWrappedYield = String?

    public func readLinesWrapped(from view: LinesView) -> String? {
        let n = view.count;
        if n == 0 {
            return .None
        }
        var idx = self % n;
        if idx < 0 { idx = idx + n }
        _linesViewAt(view: view, lineIndex: idx)
    }
}

extend Range[Int64]: LinesIndex {
    type LinesYield = LinesView

    public func readLines(from view: LinesView) -> LinesView {
        let s = self.start;
        let e = self.end;
        if s < 0 or s > e {
            fatalError("LinesView range out of bounds")
        }
        let (startByte, foundStart) = view._byteOffsetForLineIndex(lineIndex: s);
        let (endByte, foundEnd) = view._byteOffsetForLineIndex(lineIndex: e);
        if foundStart == false or foundEnd == false {
            fatalError("LinesView range out of bounds")
        }
        view._subView(startByte: startByte, endByte: endByte)
    }

    public func readLinesChecked(from view: LinesView) -> LinesView? {
        let s = self.start;
        let e = self.end;
        if s < 0 or s > e {
            return .None
        }
        let (startByte, foundStart) = view._byteOffsetForLineIndex(lineIndex: s);
        let (endByte, foundEnd) = view._byteOffsetForLineIndex(lineIndex: e);
        if foundStart == false or foundEnd == false {
            return .None
        }
        .Some(view._subView(startByte: startByte, endByte: endByte))
    }
}

extend Range[Int64]: LinesClampable {
    type LinesClampedYield = LinesView

    public func readLinesClamped(from view: LinesView) -> LinesView {
        let n = view.count;
        var s = self.start;
        var e = self.end;
        if s < 0 { s = 0 }
        if e > n { e = n }
        if s > e { s = e }
        let (startByte, _) = view._byteOffsetForLineIndex(lineIndex: s);
        let (endByte, _) = view._byteOffsetForLineIndex(lineIndex: e);
        view._subView(startByte: startByte, endByte: endByte)
    }
}

extend Range[Int64]: LinesWrappable {
    type LinesWrappedYield = LinesView

    public func readLinesWrapped(from view: LinesView) -> LinesView {
        let n = view.count;
        if n == 0 {
            return view._subView(startByte: 0, endByte: 0)
        }
        var s = self.start % n;
        if s < 0 { s = s + n }
        var span = self.end - self.start;
        if span < 0 { span = 0 }
        var e = s + span;
        if e > n { e = n }
        let (startByte, _) = view._byteOffsetForLineIndex(lineIndex: s);
        let (endByte, _) = view._byteOffsetForLineIndex(lineIndex: e);
        view._subView(startByte: startByte, endByte: endByte)
    }
}

extend ClosedRange[Int64]: LinesIndex {
    type LinesYield = LinesView

    public func readLines(from view: LinesView) -> LinesView {
        let s = self.start;
        let endExclusive = self.end + 1;
        if s < 0 or s > endExclusive {
            fatalError("LinesView range out of bounds")
        }
        let (startByte, foundStart) = view._byteOffsetForLineIndex(lineIndex: s);
        let (endByte, foundEnd) = view._byteOffsetForLineIndex(lineIndex: endExclusive);
        if foundStart == false or foundEnd == false {
            fatalError("LinesView range out of bounds")
        }
        view._subView(startByte: startByte, endByte: endByte)
    }

    public func readLinesChecked(from view: LinesView) -> LinesView? {
        let s = self.start;
        let endExclusive = self.end + 1;
        if s < 0 or s > endExclusive {
            return .None
        }
        let (startByte, foundStart) = view._byteOffsetForLineIndex(lineIndex: s);
        let (endByte, foundEnd) = view._byteOffsetForLineIndex(lineIndex: endExclusive);
        if foundStart == false or foundEnd == false {
            return .None
        }
        .Some(view._subView(startByte: startByte, endByte: endByte))
    }
}

extend ClosedRange[Int64]: LinesClampable {
    type LinesClampedYield = LinesView

    public func readLinesClamped(from view: LinesView) -> LinesView {
        let n = view.count;
        var s = self.start;
        var e = self.end + 1;
        if s < 0 { s = 0 }
        if e > n { e = n }
        if s > e { s = e }
        let (startByte, _) = view._byteOffsetForLineIndex(lineIndex: s);
        let (endByte, _) = view._byteOffsetForLineIndex(lineIndex: e);
        view._subView(startByte: startByte, endByte: endByte)
    }
}

extend ClosedRange[Int64]: LinesWrappable {
    type LinesWrappedYield = LinesView

    public func readLinesWrapped(from view: LinesView) -> LinesView {
        let n = view.count;
        if n == 0 {
            return view._subView(startByte: 0, endByte: 0)
        }
        var s = self.start % n;
        if s < 0 { s = s + n }
        var span = self.end - self.start + 1;
        if span < 0 { span = 0 }
        var e = s + span;
        if e > n { e = n }
        let (startByte, _) = view._byteOffsetForLineIndex(lineIndex: s);
        let (endByte, _) = view._byteOffsetForLineIndex(lineIndex: e);
        view._subView(startByte: startByte, endByte: endByte)
    }
}

/// Range-only index for `LinesView.substring`. See `BytesSubstringIndex`.
public protocol LinesSubstringIndex {
    func readLinesSubstring(from view: LinesView) -> String
}

extend Range[Int64]: LinesSubstringIndex {
    public func readLinesSubstring(from view: LinesView) -> String {
        view(self).toString()
    }
}

extend ClosedRange[Int64]: LinesSubstringIndex {
    public func readLinesSubstring(from view: LinesView) -> String {
        view(self).toString()
    }
}

extend RangeFrom[Int64]: LinesIndex {
    type LinesYield = LinesView

    public func readLines(from view: LinesView) -> LinesView {
        let s = self.start;
        let n = view.count;
        if s < 0 or s > n {
            fatalError("LinesView range out of bounds")
        }
        let (startByte, foundStart) = view._byteOffsetForLineIndex(lineIndex: s);
        let (endByte, foundEnd) = view._byteOffsetForLineIndex(lineIndex: n);
        if foundStart == false or foundEnd == false {
            fatalError("LinesView range out of bounds")
        }
        view._subView(startByte: startByte, endByte: endByte)
    }

    public func readLinesChecked(from view: LinesView) -> LinesView? {
        let s = self.start;
        let n = view.count;
        if s < 0 or s > n {
            return .None
        }
        let (startByte, foundStart) = view._byteOffsetForLineIndex(lineIndex: s);
        let (endByte, foundEnd) = view._byteOffsetForLineIndex(lineIndex: n);
        if foundStart == false or foundEnd == false {
            return .None
        }
        .Some(view._subView(startByte: startByte, endByte: endByte))
    }
}

extend RangeFrom[Int64]: LinesClampable {
    type LinesClampedYield = LinesView

    public func readLinesClamped(from view: LinesView) -> LinesView {
        let n = view.count;
        var s = self.start;
        if s < 0 { s = 0 }
        if s > n { s = n }
        let (startByte, _) = view._byteOffsetForLineIndex(lineIndex: s);
        let (endByte, _) = view._byteOffsetForLineIndex(lineIndex: n);
        view._subView(startByte: startByte, endByte: endByte)
    }
}

extend RangeFrom[Int64]: LinesWrappable {
    type LinesWrappedYield = LinesView

    public func readLinesWrapped(from view: LinesView) -> LinesView {
        let n = view.count;
        if n == 0 {
            return view._subView(startByte: 0, endByte: 0)
        }
        var s = self.start % n;
        if s < 0 { s = s + n }
        let (startByte, _) = view._byteOffsetForLineIndex(lineIndex: s);
        let (endByte, _) = view._byteOffsetForLineIndex(lineIndex: n);
        view._subView(startByte: startByte, endByte: endByte)
    }
}

extend RangeFrom[Int64]: LinesSubstringIndex {
    public func readLinesSubstring(from view: LinesView) -> String {
        view(self).toString()
    }
}

extend RangeUpTo[Int64]: LinesIndex {
    type LinesYield = LinesView

    public func readLines(from view: LinesView) -> LinesView {
        let e = self.end;
        if e < 0 or e > view.count {
            fatalError("LinesView range out of bounds")
        }
        let (endByte, foundEnd) = view._byteOffsetForLineIndex(lineIndex: e);
        if foundEnd == false {
            fatalError("LinesView range out of bounds")
        }
        view._subView(startByte: 0, endByte: endByte)
    }

    public func readLinesChecked(from view: LinesView) -> LinesView? {
        let e = self.end;
        if e < 0 or e > view.count {
            return .None
        }
        let (endByte, foundEnd) = view._byteOffsetForLineIndex(lineIndex: e);
        if foundEnd == false {
            return .None
        }
        .Some(view._subView(startByte: 0, endByte: endByte))
    }
}

extend RangeUpTo[Int64]: LinesClampable {
    type LinesClampedYield = LinesView

    public func readLinesClamped(from view: LinesView) -> LinesView {
        let n = view.count;
        var e = self.end;
        if e < 0 { e = 0 }
        if e > n { e = n }
        let (endByte, _) = view._byteOffsetForLineIndex(lineIndex: e);
        view._subView(startByte: 0, endByte: endByte)
    }
}

extend RangeUpTo[Int64]: LinesWrappable {
    type LinesWrappedYield = LinesView

    public func readLinesWrapped(from view: LinesView) -> LinesView {
        let n = view.count;
        if n == 0 {
            return view._subView(startByte: 0, endByte: 0)
        }
        var e = self.end % n;
        if e < 0 { e = e + n }
        let (endByte, _) = view._byteOffsetForLineIndex(lineIndex: e);
        view._subView(startByte: 0, endByte: endByte)
    }
}

extend RangeUpTo[Int64]: LinesSubstringIndex {
    public func readLinesSubstring(from view: LinesView) -> String {
        view(self).toString()
    }
}

extend RangeThrough[Int64]: LinesIndex {
    type LinesYield = LinesView

    public func readLines(from view: LinesView) -> LinesView {
        let endExclusive = self.end + 1;
        if endExclusive < 0 or endExclusive > view.count {
            fatalError("LinesView range out of bounds")
        }
        let (endByte, foundEnd) = view._byteOffsetForLineIndex(lineIndex: endExclusive);
        if foundEnd == false {
            fatalError("LinesView range out of bounds")
        }
        view._subView(startByte: 0, endByte: endByte)
    }

    public func readLinesChecked(from view: LinesView) -> LinesView? {
        let endExclusive = self.end + 1;
        if endExclusive < 0 or endExclusive > view.count {
            return .None
        }
        let (endByte, foundEnd) = view._byteOffsetForLineIndex(lineIndex: endExclusive);
        if foundEnd == false {
            return .None
        }
        .Some(view._subView(startByte: 0, endByte: endByte))
    }
}

extend RangeThrough[Int64]: LinesClampable {
    type LinesClampedYield = LinesView

    public func readLinesClamped(from view: LinesView) -> LinesView {
        let n = view.count;
        var e = self.end + 1;
        if e < 0 { e = 0 }
        if e > n { e = n }
        let (endByte, _) = view._byteOffsetForLineIndex(lineIndex: e);
        view._subView(startByte: 0, endByte: endByte)
    }
}

extend RangeThrough[Int64]: LinesWrappable {
    type LinesWrappedYield = LinesView

    public func readLinesWrapped(from view: LinesView) -> LinesView {
        let n = view.count;
        if n == 0 {
            return view._subView(startByte: 0, endByte: 0)
        }
        var e = self.end % n;
        if e < 0 { e = e + n }
        e = e + 1;
        let (endByte, _) = view._byteOffsetForLineIndex(lineIndex: e);
        view._subView(startByte: 0, endByte: endByte)
    }
}

extend RangeThrough[Int64]: LinesSubstringIndex {
    public func readLinesSubstring(from view: LinesView) -> String {
        view(self).toString()
    }
}

// ============================================================================
// TYPED INDEX CONFORMANCES
// ============================================================================

// -- ByteIndex on BytesView --------------------------------------------------

extend ByteIndex: BytesIndex {
    type BytesYield = UInt8

    public func readBytes(from view: BytesView) -> UInt8 {
        if self.value < 0 or self.value >= view.length {
            fatalError("BytesView index out of bounds")
        }
        view._readByteRaw(index: self.value)
    }

    public func readBytesChecked(from view: BytesView) -> UInt8? {
        if self.value >= 0 and self.value < view.length {
            .Some(view._readByteRaw(index: self.value))
        } else {
            .None
        }
    }

    public func readBytesUnchecked(from view: BytesView) -> UInt8 {
        view._readByteRaw(index: self.value)
    }
}

// -- CharIndex on CharsView --------------------------------------------------

extend CharIndex: CharsIndex {
    type CharsYield = Char

    public func readChars(from view: CharsView) -> Char {
        if self.byteOffset < 0 or self.byteOffset >= view.length {
            fatalError("CharsView index out of bounds")
        }
        let result = decodeUtf8(view.ptr, view.length, at: self.byteOffset);
        match result {
            .Some(d) => d.char,
            .None => Char(unchecked: 0xFFFD)
        }
    }

    public func readCharsChecked(from view: CharsView) -> Char? {
        if self.byteOffset < 0 or self.byteOffset >= view.length {
            return .None
        }
        let result = decodeUtf8(view.ptr, view.length, at: self.byteOffset);
        match result {
            .Some(d) => .Some(d.char),
            .None => .Some(Char(unchecked: 0xFFFD))
        }
    }
}

// -- View range-slice methods ------------------------------------------------

extend BytesView {
    /// Returns a `StringSlice` covering the byte range `[start, end)`.
    public func slice(from start: ByteIndex, to end: ByteIndex) -> StringSlice {
        self.slice.subslice(from: self.slice.start + start.value, to: self.slice.start + end.value)
    }

    /// Byte index of the first byte.
    public var startIndex: ByteIndex { ByteIndex(0) }

    /// Byte index one past the last byte.
    public var endIndex: ByteIndex { ByteIndex(self.length) }
}

extend CharsView {
    /// Returns a `StringSlice` covering `[start, end)` by byte offset.
    public func slice(from start: CharIndex, to end: CharIndex) -> StringSlice {
        self.slice.subslice(from: self.slice.start + start.byteOffset, to: self.slice.start + end.byteOffset)
    }

    /// Char index at byte 0 (the first code point).
    public var startIndex: CharIndex { CharIndex(0) }

    /// Char index at the end (one past the last byte).
    public var endIndex: CharIndex { CharIndex(self.length) }
}

extend GraphemesView {
    /// Returns a `StringSlice` covering `[start, end)` by byte offset.
    public func slice(from start: GraphemeIndex, to end: GraphemeIndex) -> StringSlice {
        self.slice.subslice(from: self.slice.start + start.byteOffset, to: self.slice.start + end.byteOffset)
    }

    /// Grapheme index at byte 0.
    public var startIndex: GraphemeIndex { GraphemeIndex(0) }

    /// Grapheme index at the end (one past the last byte).
    public var endIndex: GraphemeIndex { GraphemeIndex(self.length) }
}

extend LinesView {
    /// Returns a `StringSlice` covering `[start, end)` by byte offset.
    public func slice(from start: LineIndex, to end: LineIndex) -> StringSlice {
        self.slice.subslice(from: self.slice.start + start.byteOffset, to: self.slice.start + end.byteOffset)
    }

    /// Line index at byte 0.
    public var startIndex: LineIndex { LineIndex(0) }

    /// Line index at the end (one past the last byte).
    public var endIndex: LineIndex { LineIndex(self.length) }
}

// ============================================================================
// SEARCH METHODS RETURNING TYPED INDICES
// ============================================================================

extend BytesView {
    /// Returns the index of the first occurrence of `byte`, or `.None`.
    public func firstIndex(of byte: UInt8) -> ByteIndex? {
        for i in 0..<self.length {
            if self._readByteRaw(index: i) == byte {
                return .Some(ByteIndex(i))
            }
        }
        .None
    }

    /// Returns the index of the last occurrence of `byte`, or `.None`.
    public func lastIndex(of byte: UInt8) -> ByteIndex? {
        var i = self.length - 1;
        while i >= 0 {
            if self._readByteRaw(index: i) == byte {
                return .Some(ByteIndex(i))
            }
            i = i - 1
        }
        .None
    }
}

extend CharsView {
    /// Returns the index of the first occurrence of `c`, or `.None`.
    public func firstIndex(of c: Char) -> CharIndex? {
        var byteIdx: Int64 = 0;
        while byteIdx < self.length {
            let result = decodeUtf8(self.ptr, self.length, at: byteIdx);
            if let .Some(decoded) = result {
                if decoded.char == c {
                    return .Some(CharIndex(byteIdx))
                }
                byteIdx = byteIdx + decoded.bytesConsumed
            } else {
                byteIdx = byteIdx + 1
            }
        }
        .None
    }

    /// Returns the index of the last occurrence of `c`, or `.None`.
    public func lastIndex(of c: Char) -> CharIndex? {
        var lastFound: CharIndex? = .None;
        var byteIdx: Int64 = 0;
        while byteIdx < self.length {
            let result = decodeUtf8(self.ptr, self.length, at: byteIdx);
            if let .Some(decoded) = result {
                if decoded.char == c {
                    lastFound = .Some(CharIndex(byteIdx))
                }
                byteIdx = byteIdx + decoded.bytesConsumed
            } else {
                byteIdx = byteIdx + 1
            }
        }
        lastFound
    }

    /// Resolves the n-th code point to its byte offset. O(n).
    public func index(at position: Int64) -> CharIndex? {
        let (offset, found) = self._byteOffsetForCharIndex(charIndex: position);
        if found { .Some(CharIndex(offset)) } else { .None }
    }

    /// Returns the index of the first code point matching `predicate`, or `.None`.
    public func firstIndex(where predicate: (Char) -> Bool) -> CharIndex? {
        var byteIdx: Int64 = 0;
        while byteIdx < self.length {
            let result = decodeUtf8(self.ptr, self.length, at: byteIdx);
            if let .Some(decoded) = result {
                if predicate(decoded.char) {
                    return .Some(CharIndex(byteIdx))
                }
                byteIdx = byteIdx + decoded.bytesConsumed
            } else {
                byteIdx = byteIdx + 1
            }
        }
        .None
    }

    /// Returns the index of the last code point matching `predicate`, or `.None`.
    public func lastIndex(where predicate: (Char) -> Bool) -> CharIndex? {
        var lastFound: CharIndex? = .None;
        var byteIdx: Int64 = 0;
        while byteIdx < self.length {
            let result = decodeUtf8(self.ptr, self.length, at: byteIdx);
            if let .Some(decoded) = result {
                if predicate(decoded.char) {
                    lastFound = .Some(CharIndex(byteIdx))
                }
                byteIdx = byteIdx + decoded.bytesConsumed
            } else {
                byteIdx = byteIdx + 1
            }
        }
        lastFound
    }
}

extend GraphemesView {
    /// Returns the index of the first grapheme matching `predicate`, or `.None`.
    public func firstIndex(where predicate: (Grapheme) -> Bool) -> GraphemeIndex? {
        var byteIdx: Int64 = 0;
        var it = self.iter();
        while let .Some(g) = it.next() {
            if predicate(g) {
                return .Some(GraphemeIndex(byteIdx))
            }
            byteIdx = byteIdx + g.utf8Length()
        }
        .None
    }

    /// Resolves the n-th grapheme cluster to its byte offset. O(n) —
    /// walks the segmenter from the start.
    public func index(at position: Int64) -> GraphemeIndex? {
        let (offset, found) = self._byteOffsetForGraphemeIndex(graphemeIndex: position);
        if found { .Some(GraphemeIndex(offset)) } else { .None }
    }
}

extend LinesView {
    /// Resolves the n-th line to its byte offset. O(n) — scans for
    /// line terminators from the start.
    public func index(at position: Int64) -> LineIndex? {
        let (offset, found) = self._byteOffsetForLineIndex(lineIndex: position);
        if found { .Some(LineIndex(offset)) } else { .None }
    }
}

// ============================================================================
// INDEX ADVANCEMENT
// ============================================================================

extend GraphemeIndex {
    /// Advances by `n` grapheme clusters. Requires the source slice to
    /// run the UAX #29 segmenter forward. O(n) in graphemes advanced.
    ///
    /// # Examples
    ///
    /// ```
    /// let s = "héllo";
    /// let idx = s.graphemes.startIndex;    // byte 0
    /// let next = idx.advance(by: 2, from: s.asSlice());
    /// // Skipped 'h' (1 byte) and 'é' (2 bytes) → byte 3
    /// ```
    public func advance(by n: Int64, from source: StringSlice) -> GraphemeIndex {
        let rawPtr: lang.ptr[lang.i8] = lang.cast_ptr[_, lang.i8](source._rawPtr().offset(by: source.start).asRaw().raw);
        let length = source.byteCount;
        var offset = self.byteOffset;
        var remaining = n;

        var charsIter = CharsIterator(ptr: rawPtr, length: length, byteIndex: offset);
        var graphemeIter = GraphemesIterator(charsIter);

        while remaining > 0 {
            if let .Some(g) = graphemeIter.next() {
                offset = offset + g.utf8Length();
                remaining = remaining - 1
            } else {
                break
            }
        }

        GraphemeIndex(offset)
    }
}

extend LineIndex {
    /// Advances by `n` lines. Scans for line terminators (`\n`, `\r\n`,
    /// `\r`) from the current byte offset. O(n) in lines advanced.
    ///
    /// # Examples
    ///
    /// ```
    /// let s = "a\nb\nc";
    /// let idx = s.lines.startIndex;       // byte 0
    /// let second = idx.advance(by: 1, from: s.asSlice());
    /// // second.byteOffset == 2 (past "a\n")
    /// ```
    public func advance(by n: Int64, from source: StringSlice) -> LineIndex {
        let rawPtr: lang.ptr[lang.i8] = lang.cast_ptr[_, lang.i8](source._rawPtr().offset(by: source.start).asRaw().raw);
        let length = source.byteCount;
        var offset = self.byteOffset;
        var remaining = n;

        while remaining > 0 and offset < length {
            let rawOffset: lang.i64 = offset.raw;
            let bytePtr: lang.ptr[lang.i8] = lang.ptr_offset[lang.i8](rawPtr, rawOffset);
            let byte: lang.i8 = lang.ptr_read(bytePtr);
            let byteVal: lang.i32 = lang.cast_i8_i32(byte);
            let unsignedByte: lang.i32 = lang.i32_and(byteVal, 0xFF);

            if Bool(boolLiteral: lang.i32_eq(unsignedByte, 10)) {
                offset = offset + 1;
                remaining = remaining - 1
            } else if Bool(boolLiteral: lang.i32_eq(unsignedByte, 13)) {
                offset = offset + 1;
                if offset < length {
                    let nextOffset: lang.i64 = offset.raw;
                    let nextBytePtr: lang.ptr[lang.i8] = lang.ptr_offset[lang.i8](rawPtr, nextOffset);
                    let nextByte: lang.i8 = lang.ptr_read(nextBytePtr);
                    let nextByteVal: lang.i32 = lang.cast_i8_i32(nextByte);
                    let nextUnsigned: lang.i32 = lang.i32_and(nextByteVal, 0xFF);
                    if Bool(boolLiteral: lang.i32_eq(nextUnsigned, 10)) {
                        offset = offset + 1
                    }
                }
                remaining = remaining - 1
            } else {
                offset = offset + 1
            }
        }

        LineIndex(offset)
    }
}

// ============================================================================
// INDEXED ITERATORS
// ============================================================================

/// Iterator yielding `(CharIndex, Char)` pairs — the byte offset of each
/// code point alongside the decoded character. Useful when you need to
/// know where each char starts in the buffer.
public struct IndexedCharsIterator: Iterator {
    type Item = (CharIndex, Char)

    private var ptr: lang.ptr[lang.i8]
    private var length: Int64
    private var byteIndex: Int64

    public init(ptr ptr: lang.ptr[lang.i8], length length: Int64) {
        self.ptr = ptr;
        self.length = length;
        self.byteIndex = 0;
    }

    public mutating func next() -> (CharIndex, Char)? {
        if self.byteIndex >= self.length { return .None }

        let idx = CharIndex(self.byteIndex);
        let result = decodeUtf8(self.ptr, self.length, at: self.byteIndex);
        if let .Some(decoded) = result {
            self.byteIndex = self.byteIndex + decoded.bytesConsumed;
            .Some((idx, decoded.char))
        } else {
            self.byteIndex = self.byteIndex + 1;
            let replacementValue = UInt32(raw: 0xFFFD);
            .Some((idx, Char(unchecked: replacementValue)))
        }
    }
}

/// Iterator yielding `(GraphemeIndex, Grapheme)` pairs — the byte offset
/// of each grapheme cluster alongside the grapheme value.
public struct IndexedGraphemesIterator: Iterator {
    type Item = (GraphemeIndex, Grapheme)

    private var inner: GraphemesIterator
    private var byteOffset: Int64

    public init(inner inner: GraphemesIterator) {
        self.inner = inner;
        self.byteOffset = 0;
    }

    public mutating func next() -> (GraphemeIndex, Grapheme)? {
        if let .Some(g) = self.inner.next() {
            let idx = GraphemeIndex(self.byteOffset);
            self.byteOffset = self.byteOffset + g.utf8Length();
            .Some((idx, g))
        } else {
            .None
        }
    }
}

/// Iterator yielding `(LineIndex, String)` pairs — the byte offset of each
/// line's start alongside the line content (without terminator).
public struct IndexedLinesIterator: Iterator {
    type Item = (LineIndex, String)

    private var ptr: lang.ptr[lang.i8]
    private var length: Int64
    private var byteIndex: Int64
    private var done: Bool

    public init(ptr ptr: lang.ptr[lang.i8], length length: Int64) {
        self.ptr = ptr;
        self.length = length;
        self.byteIndex = 0;
        self.done = false;
    }

    public mutating func next() -> (LineIndex, String)? {
        if self.done or self.byteIndex >= self.length {
            return .None
        }

        let start = self.byteIndex;
        let idx = LineIndex(start);
        var foundNewline: Bool = false;
        var lineEnd: Int64 = self.byteIndex;

        while self.byteIndex < self.length and foundNewline == false {
            let rawOffset: lang.i64 = self.byteIndex.raw;
            let bytePtr: lang.ptr[lang.i8] = lang.ptr_offset[lang.i8](self.ptr, rawOffset);
            let byte: lang.i8 = lang.ptr_read(bytePtr);
            let byteVal: lang.i32 = lang.cast_i8_i32(byte);
            let unsignedByte: lang.i32 = lang.i32_and(byteVal, 0xFF);

            if Bool(boolLiteral: lang.i32_eq(unsignedByte, 10)) {
                lineEnd = self.byteIndex;
                self.byteIndex = self.byteIndex + 1;
                foundNewline = true
            } else if Bool(boolLiteral: lang.i32_eq(unsignedByte, 13)) {
                lineEnd = self.byteIndex;
                self.byteIndex = self.byteIndex + 1;
                if self.byteIndex < self.length {
                    let nextOffset: lang.i64 = self.byteIndex.raw;
                    let nextBytePtr: lang.ptr[lang.i8] = lang.ptr_offset[lang.i8](self.ptr, nextOffset);
                    let nextByte: lang.i8 = lang.ptr_read(nextBytePtr);
                    let nextByteVal: lang.i32 = lang.cast_i8_i32(nextByte);
                    let nextUnsigned: lang.i32 = lang.i32_and(nextByteVal, 0xFF);
                    if Bool(boolLiteral: lang.i32_eq(nextUnsigned, 10)) {
                        self.byteIndex = self.byteIndex + 1
                    }
                }
                foundNewline = true
            } else {
                self.byteIndex = self.byteIndex + 1
            }
        }

        if foundNewline {
            return .Some((idx, self._createSubstring(start, lineEnd)))
        }

        if start < self.length {
            self.done = true;
            return .Some((idx, self._createSubstring(start, self.length)))
        }

        let none: (LineIndex, String)? = .None;
        none
    }

    private func _createSubstring(start: Int64, end: Int64) -> String {
        let count = end - start;
        if count <= 0 {
            return String()
        }
        let srcAt: lang.ptr[lang.i8] = lang.ptr_offset[lang.i8](self.ptr, start.raw);
        String.fromRawBytes(srcAt, count)
    }
}

extend CharsView {
    /// Returns an iterator yielding `(CharIndex, Char)` pairs.
    public func indexedIter() -> IndexedCharsIterator {
        IndexedCharsIterator(ptr: self.ptr, length: self.length)
    }
}

extend GraphemesView {
    /// Returns an iterator yielding `(GraphemeIndex, Grapheme)` pairs.
    public func indexedIter() -> IndexedGraphemesIterator {
        IndexedGraphemesIterator(inner: self.iter())
    }
}

extend LinesView {
    /// Returns an iterator yielding `(LineIndex, String)` pairs.
    public func indexedIter() -> IndexedLinesIterator {
        IndexedLinesIterator(ptr: self.ptr, length: self.length)
    }
}

// ============================================================================
// REVERSED CHARS VIEW
// ============================================================================

/// Iterator that yields code points back-to-front by walking backward
/// through UTF-8 continuation bytes to find each leading byte, then
/// decoding forward.
public struct ReversedCharsIterator: Iterator {
    type Item = Char

    private var ptr: lang.ptr[lang.i8]
    private var length: Int64
    private var byteIndex: Int64

    public init(ptr ptr: lang.ptr[lang.i8], length length: Int64) {
        self.ptr = ptr;
        self.length = length;
        self.byteIndex = length;
    }

    public mutating func next() -> Char? {
        if self.byteIndex <= 0 { return .None }

        // Walk backwards past continuation bytes (10xxxxxx)
        var i = self.byteIndex - 1;
        while i > 0 {
            let rawOffset: lang.i64 = i.raw;
            let bytePtr: lang.ptr[lang.i8] = lang.ptr_offset[lang.i8](self.ptr, rawOffset);
            let signedByte: lang.i8 = lang.ptr_read(bytePtr);
            let v: lang.i32 = lang.cast_i8_i32(signedByte);
            if lang.i32_ne(lang.i32_and(v, 0xC0), 0x80) {
                break
            }
            i = i - 1
        }

        self.byteIndex = i;
        let result = decodeUtf8(self.ptr, self.length, at: i);
        if let .Some(decoded) = result {
            .Some(decoded.char)
        } else {
            .Some(Char(unchecked: UInt32(raw: 0xFFFD)))
        }
    }
}

/// A reversed view over the code points in a string. Iterates characters
/// back-to-front without allocating.
///
/// # Examples
///
/// ```
/// let view = "abc".chars.reversed;
/// view.first();    // Some('c')
/// view.count;      // 3
/// ```
public struct ReversedCharsView: Iterable, Cloneable {
    type Item = Char
    type TargetIterator = ReversedCharsIterator

    fileprivate var slice: StringSlice
    fileprivate var ptr: lang.ptr[lang.i8]
    fileprivate var length: Int64

    public init(slice slice: StringSlice) {
        self.slice = slice;
        self.ptr = lang.cast_ptr[_, lang.i8](slice._rawPtr().offset(by: slice.start).asRaw().raw);
        self.length = slice.byteCount;
    }

    public func clone() -> ReversedCharsView { ReversedCharsView(slice: self.slice.clone()) }

    public func iter() -> ReversedCharsIterator {
        ReversedCharsIterator(ptr: self.ptr, length: self.length)
    }

    /// Number of code points. O(n).
    public var count: Int64 {
        var n: Int64 = 0;
        var it = self.iter();
        while let .Some(_) = it.next() {
            n = n + 1
        }
        n
    }

    public var isEmpty: Bool { self.length == 0 }

    /// The first element of the reversed view (= last char of the source).
    public var first: Char? {
        var it = self.iter();
        it.next()
    }
}

extend CharsView {
    /// A reversed view that iterates code points back-to-front.
    public var reversed: ReversedCharsView { ReversedCharsView(slice: self.slice) }

    /// The first code point, or `None` if the view is empty.
    public var first: Char? {
        var it = self.iter();
        it.next()
    }

    /// The last code point, or `None` if the view is empty.
    public var last: Char? {
        self.reversed.first
    }
}

extend GraphemesView {
    /// The first grapheme cluster, or `None` if the view is empty.
    ///
    /// O(1) in practice — decodes one cluster from the start.
    public var first: Grapheme? {
        var it = self.iter();
        it.next()
    }

    /// The last grapheme cluster, or `None` if the view is empty.
    ///
    /// O(n) — walks the entire string through the segmenter.
    public var last: Grapheme? {
        var it = self.iter();
        var result: Grapheme? = .None;
        while let .Some(g) = it.next() {
            result = .Some(g)
        }
        result
    }
}

extend LinesView {
    /// The first line (without terminator), or `None` if the view is empty.
    ///
    /// O(first line length) — scans for the first terminator.
    public var first: String? {
        var it = self.iter();
        it.next()
    }
}

// ============================================================================
// SPLIT VIEW
// ============================================================================

/// Iterator that yields `StringSlice` segments produced by splitting on a
/// fixed separator. Zero-copy: each yielded slice is a window into the
/// original source buffer.
public struct SplitViewIterator: Iterator, Cloneable {
    type Item = StringSlice

    fileprivate var slice: StringSlice
    fileprivate var separator: String
    fileprivate var sourcePtr: Pointer[UInt8]
    fileprivate var sourceLen: Int64
    fileprivate var index: Int64
    fileprivate var done: Bool

    public init(slice slice: StringSlice, separator separator: String) {
        self.slice = slice;
        self.separator = separator;
        self.sourcePtr = slice._rawPtr().offset(by: slice.start);
        self.sourceLen = slice.byteCount;
        self.index = 0;
        self.done = false;
    }

    fileprivate init(slice slice: StringSlice, separator separator: String, index index: Int64, done done: Bool) {
        self.slice = slice;
        self.separator = separator;
        self.sourcePtr = slice._rawPtr().offset(by: slice.start);
        self.sourceLen = slice.byteCount;
        self.index = index;
        self.done = done;
    }

    public func clone() -> SplitViewIterator {
        SplitViewIterator(slice: self.slice.clone(), separator: self.separator.clone(), index: self.index, done: self.done)
    }

    public mutating func next() -> StringSlice? {
        if self.done { return .None }

        let start = self.index;
        let sepLen = self.separator.asSlice().byteCount;

        if sepLen == 0 {
            // Empty separator — split per code point
            if self.index >= self.sourceLen {
                self.done = true;
                return .None
            }
            let rawPtr: lang.ptr[lang.i8] = lang.cast_ptr[_, lang.i8](self.sourcePtr.asRaw().raw);
            let result = decodeUtf8(rawPtr, self.sourceLen, at: self.index);
            if let .Some(decoded) = result {
                self.index = self.index + decoded.bytesConsumed;
                return .Some(self.slice.subslice(from: self.slice.start + start, to: self.slice.start + self.index))
            }
            self.done = true;
            return .None
        }

        // Search for separator via memmem
        let remaining = self.sourceLen - self.index;
        if remaining >= sepLen {
            let base = self.sourcePtr.offset(by: self.index).asRaw();
            let sepSlice = self.separator.asSlice();
            let needle = sepSlice._rawPtr().offset(by: sepSlice.start).asRaw();
            let result = memmem(base, remaining, needle, sepLen);
            if result.isNull == false {
                let diff: lang.i64 = lang.i64_sub(result.address.raw, base.address.raw);
                let matchIndex = self.index + Int64(intLiteral: diff);
                self.index = matchIndex + sepLen;
                return .Some(self.slice.subslice(from: self.slice.start + start, to: self.slice.start + matchIndex))
            }
        }

        // No more separators — return trailing remainder
        self.done = true;
        if start < self.sourceLen {
            return .Some(self.slice.subslice(from: self.slice.start + start, to: self.slice.end))
        }
        .None
    }
}

/// Lazy view over the segments of a string split on a fixed separator.
///
/// Each segment is a zero-copy `StringSlice` into the original buffer.
/// Use `iter()` for one-pass iteration, or `first()`/`last()`/`collect()`
/// for targeted access.
///
/// # Examples
///
/// ```
/// let view = "a,b,c".asSlice().split(",");
/// view.first();            // Some("a")
/// view.count;              // 3
/// view.collect();          // [StringSlice("a"), StringSlice("b"), StringSlice("c")]
/// ```
public struct SplitView: Iterable, Cloneable {
    type Item = StringSlice
    type TargetIterator = SplitViewIterator

    fileprivate var slice: StringSlice
    fileprivate var separator: String

    public init(slice slice: StringSlice, separator separator: String) {
        self.slice = slice;
        self.separator = separator;
    }

    public func clone() -> SplitView {
        SplitView(slice: self.slice.clone(), separator: self.separator.clone())
    }

    public func iter() -> SplitViewIterator {
        SplitViewIterator(slice: self.slice.clone(), separator: self.separator.clone())
    }

    /// True when the source slice is empty.
    public var isEmpty: Bool { self.slice.isEmpty }

    /// Number of segments. O(n) — iterates once to count.
    public var count: Int64 {
        var it = self.iter();
        var n: Int64 = 0;
        while let .Some(_) = it.next() {
            n = n + 1
        }
        n
    }

    /// The first segment, or `.None` if empty.
    public var first: StringSlice? {
        var it = self.iter();
        it.next()
    }

    /// The last segment, or `.None` if empty.
    public var last: StringSlice? {
        var it = self.iter();
        var result: StringSlice? = .None;
        while let .Some(segment) = it.next() {
            result = .Some(segment)
        }
        result
    }

    /// Collects all segments into an array.
    public func collect() -> Array[StringSlice] {
        var result = Array[StringSlice]();
        for segment in self {
            result.append(segment)
        }
        result
    }
}

// ============================================================================
// SPLIT WHERE VIEW
// ============================================================================

/// Iterator that yields `StringSlice` segments produced by splitting at
/// every code point matching a predicate. The matching character is not
/// included in any segment.
public struct SplitWhereViewIterator: Iterator, Cloneable {
    type Item = StringSlice

    fileprivate var slice: StringSlice
    fileprivate var predicate: (Char) -> Bool
    fileprivate var sourcePtr: Pointer[UInt8]
    fileprivate var sourceLen: Int64
    fileprivate var index: Int64
    fileprivate var done: Bool

    public init(slice slice: StringSlice, consuming where predicate: (Char) -> Bool) {
        self.slice = slice;
        self.predicate = predicate;
        self.sourcePtr = slice._rawPtr().offset(by: slice.start);
        self.sourceLen = slice.byteCount;
        self.index = 0;
        self.done = false;
    }

    fileprivate init(slice slice: StringSlice, consuming where predicate: (Char) -> Bool, index index: Int64, done done: Bool) {
        self.slice = slice;
        self.predicate = predicate;
        self.sourcePtr = slice._rawPtr().offset(by: slice.start);
        self.sourceLen = slice.byteCount;
        self.index = index;
        self.done = done;
    }

    public func clone() -> SplitWhereViewIterator {
        SplitWhereViewIterator(slice: self.slice.clone(), where: self.predicate, index: self.index, done: self.done)
    }

    public mutating func next() -> StringSlice? {
        if self.done { return .None }

        let start = self.index;
        let rawPtr: lang.ptr[lang.i8] = lang.cast_ptr[_, lang.i8](self.sourcePtr.asRaw().raw);

        while self.index < self.sourceLen {
            let result = decodeUtf8(rawPtr, self.sourceLen, at: self.index);
            if let .Some(decoded) = result {
                if self.predicate(decoded.char) {
                    let matchIndex = self.index;
                    self.index = self.index + decoded.bytesConsumed;
                    return .Some(self.slice.subslice(from: self.slice.start + start, to: self.slice.start + matchIndex))
                }
                self.index = self.index + decoded.bytesConsumed
            } else {
                self.index = self.index + 1
            }
        }

        // No more matches — return remainder
        self.done = true;
        if start < self.sourceLen {
            return .Some(self.slice.subslice(from: self.slice.start + start, to: self.slice.end))
        }
        .None
    }
}

/// Lazy view over the segments of a string split at every code point
/// matching a predicate. The matching characters are excluded from segments.
///
/// # Examples
///
/// ```
/// let view = "hello world".asSlice().split { (c) in c == Char(" ") };
/// view.first();    // Some("hello")
/// view.count;      // 2
/// ```
public struct SplitWhereView: Iterable, Cloneable {
    type Item = StringSlice
    type TargetIterator = SplitWhereViewIterator

    fileprivate var slice: StringSlice
    fileprivate var predicate: (Char) -> Bool

    public init(slice slice: StringSlice, consuming where predicate: (Char) -> Bool) {
        self.slice = slice;
        self.predicate = predicate;
    }

    public func clone() -> SplitWhereView {
        SplitWhereView(slice: self.slice.clone(), where: self.predicate)
    }

    public func iter() -> SplitWhereViewIterator {
        SplitWhereViewIterator(slice: self.slice.clone(), where: self.predicate)
    }

    /// True when the source slice is empty.
    public var isEmpty: Bool { self.slice.isEmpty }

    /// Number of segments. O(n) — iterates once to count.
    public var count: Int64 {
        var it = self.iter();
        var n: Int64 = 0;
        while let .Some(_) = it.next() {
            n = n + 1
        }
        n
    }

    /// The first segment, or `.None` if empty.
    public var first: StringSlice? {
        var it = self.iter();
        it.next()
    }

    /// The last segment, or `.None` if empty.
    public var last: StringSlice? {
        var it = self.iter();
        var result: StringSlice? = .None;
        while let .Some(segment) = it.next() {
            result = .Some(segment)
        }
        result
    }

    /// Collects all segments into an array.
    public func collect() -> Array[StringSlice] {
        var result = Array[StringSlice]();
        for segment in self {
            result.append(segment)
        }
        result
    }
}
