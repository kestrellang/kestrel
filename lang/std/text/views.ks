// String views for different representations

module std.text

import std.core.(Bool, Equatable, Comparable, Ordering, Range, ClosedRange, fatalError)
import std.numeric.(Int64, UInt8, UInt32)
import std.result.(Optional)
import std.iter.(Iterator, Iterable)
import std.text.(Char, Grapheme, Byte, decodeUtf8, String)
import std.text.unicode.(GraphemeBreakProperty, graphemeBreakProperty, shouldBreakBetween)
import std.collections.(Array)

// TODO: all view structs store lang.ptr[lang.i8] and use lang.ptr_offset/lang.ptr_read
// directly. Migrate fields to RawPointer and byte access to RawPointer.offset/cast
// once the UTF-8 codec (char.ks) accepts RawPointer instead of lang.ptr[lang.i8].

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
public struct BytesView: Iterable {
    /// The element type yielded by iteration — always `UInt8`.
    type Item = UInt8
    /// The iterator type returned by `iter()`.
    type TargetIterator = BytesIterator

    private var ptr: lang.ptr[lang.i8]
    private var length: Int64

    /// @name From Pointer
    /// Constructs a bytes view from a raw pointer and a byte count.
    ///
    /// Prefer `someString.bytes` over calling this directly.
    ///
    /// # Safety
    ///
    /// `ptr` must point to `length` valid bytes that remain live for as
    /// long as the view is used.
    public init(ptr ptr: lang.ptr[lang.i8], length length: Int64) {
        self.ptr = ptr;
        self.length = length;
    }

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
    public func substring[I](range: I) -> String where I: BytesSubstringIndex {
        range.readBytesSubstring(from: self)
    }

    /// Internal: build a sub-view over byte range `[startByte, endByte)`.
    fileprivate func _subView(startByte startByte: Int64, endByte endByte: Int64) -> BytesView {
        let rawOffset: lang.i64 = startByte.raw;
        let newPtr: lang.ptr[lang.i8] = lang.ptr_offset[lang.i8](self.ptr, rawOffset);
        BytesView(ptr: newPtr, length: endByte - startByte)
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
            .Some(Char(replacementValue))
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
public struct CharsView: Iterable {
    /// The element type yielded by iteration — always `Char`.
    type Item = Char
    /// The iterator type returned by `iter()`.
    type TargetIterator = CharsIterator

    private var ptr: lang.ptr[lang.i8]
    private var length: Int64

    /// @name From Pointer
    /// Constructs a chars view from a raw pointer and a byte length.
    ///
    /// Prefer `someString.chars` over calling this directly.
    ///
    /// # Safety
    ///
    /// `ptr` must point to `length` valid UTF-8 bytes that remain live
    /// for the view's lifetime.
    public init(ptr ptr: lang.ptr[lang.i8], length length: Int64) {
        self.ptr = ptr;
        self.length = length;
    }

    /// Returns a `CharsIterator` positioned at byte 0.
    ///
    /// Each call returns a fresh iterator; the view itself is reusable.
    public func iter() -> CharsIterator {
        CharsIterator(ptr: self.ptr, length: self.length, byteIndex: 0)
    }

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
        let rawOffset: lang.i64 = startByte.raw;
        let newPtr: lang.ptr[lang.i8] = lang.ptr_offset[lang.i8](self.ptr, rawOffset);
        CharsView(ptr: newPtr, length: endByte - startByte)
    }

    /// Materializes the view as an owned `String`. O(n) — copies bytes.
    public func toString() -> String {
        _copyByteRange(ptr: self.ptr, startByte: 0, endByte: self.length)
    }

    /// Convenience: dispatches to a `CharsSubstringIndex` to produce
    /// an owned `String` covering the requested code-point range.
    /// Equivalent to `self(range).toString()` for both `Range[Int64]`
    /// and `ClosedRange[Int64]`.
    public func substring[I](range: I) -> String where I: CharsSubstringIndex {
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
public struct GraphemesView: Iterable {
    /// The element type yielded by iteration — always `Grapheme`.
    type Item = Grapheme
    /// The iterator type returned by `iter()`.
    type TargetIterator = GraphemesIterator

    private var ptr: lang.ptr[lang.i8]
    private var length: Int64

    /// @name From Pointer
    /// Constructs a graphemes view from a raw pointer and a byte length.
    ///
    /// Prefer `someString.graphemes` over calling this directly.
    ///
    /// # Safety
    ///
    /// `ptr` must point to `length` valid UTF-8 bytes that remain live
    /// for the view's lifetime.
    public init(ptr ptr: lang.ptr[lang.i8], length length: Int64) {
        self.ptr = ptr;
        self.length = length;
    }

    /// Returns a `GraphemesIterator` positioned at byte 0.
    public func iter() -> GraphemesIterator {
        GraphemesIterator(CharsIterator(ptr: self.ptr, length: self.length, byteIndex: 0))
    }

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
        let rawOffset: lang.i64 = startByte.raw;
        let newPtr: lang.ptr[lang.i8] = lang.ptr_offset[lang.i8](self.ptr, rawOffset);
        GraphemesView(ptr: newPtr, length: endByte - startByte)
    }

    /// Materializes the view as an owned `String`. O(n) — copies bytes.
    public func toString() -> String {
        _copyByteRange(ptr: self.ptr, startByte: 0, endByte: self.length)
    }

    /// Convenience: dispatches to a `GraphemesSubstringIndex` to
    /// produce an owned `String` covering the requested cluster range.
    /// Equivalent to `self(range).toString()` for both `Range[Int64]`
    /// and `ClosedRange[Int64]`.
    public func substring[I](range: I) -> String where I: GraphemesSubstringIndex {
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
public struct LinesView: Iterable {
    /// The element type yielded by iteration — always `String`.
    type Item = String
    /// The iterator type returned by `iter()`.
    type TargetIterator = LinesIterator

    private var ptr: lang.ptr[lang.i8]
    private var length: Int64

    /// @name From Pointer
    /// Constructs a lines view from a raw pointer and a byte length.
    ///
    /// Prefer `someString.lines` over calling this directly.
    ///
    /// # Safety
    ///
    /// `ptr` must point to `length` valid bytes that remain live for
    /// the view's lifetime.
    public init(ptr ptr: lang.ptr[lang.i8], length length: Int64) {
        self.ptr = ptr;
        self.length = length;
    }

    /// Returns a `LinesIterator` positioned at byte 0.
    public func iter() -> LinesIterator {
        LinesIterator(ptr: self.ptr, length: self.length, byteIndex: 0, done: false)
    }

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
    public func substring[I](range: I) -> String where I: LinesSubstringIndex {
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
        let rawOffset: lang.i64 = startByte.raw;
        let newPtr: lang.ptr[lang.i8] = lang.ptr_offset[lang.i8](self.ptr, rawOffset);
        LinesView(ptr: newPtr, length: endByte - startByte)
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
