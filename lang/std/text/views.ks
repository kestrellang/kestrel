// String views for different representations

module std.text

import std.core.(Bool, Equatable, Comparable, Ordering, Range, ClosedRange, RangeFrom, RangeUpTo, RangeThrough, Cloneable, fatalError)
import std.numeric.(Int64, UInt8, UInt32)
import std.result.(Optional)
import std.iter.(Iterator, Iterable)
import std.text.(Char, Grapheme, decodeUtf8, String, StringSlice, LineIndex, Formattable, Formatter, FormatOptions)
import std.text.unicode.(GraphemeBreakProperty, graphemeBreakProperty, shouldBreakBetween)
import std.memory.(Pointer, UnsafeSlice)
import std.collections.(Array, SeqRange, RangeBound)
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
/// A `(source, length, index)` triple: the borrowed `StringSlice` plus
/// the cursor and total-length pair the iterator advances through.
///
/// # Memory Model
///
/// Borrows the source `String` and reads its current buffer on every
/// `next()`, so it can't outlive the string and never reads freed memory.
public struct BytesIterator: Iterator {
    /// The element type yielded by `next()` — always `UInt8`.
    type Item = UInt8

    let source: &StringSlice
    // Re-read on every access: the string may have changed since.
    private var ptr: lang.ptr[lang.i8] { self.source._bytes() }
    fileprivate var length: Int64
    fileprivate var index: Int64

    /// Returns the next byte, or `None` once `index` reaches `length`.
    ///
    /// Each call reads one byte and advances the cursor by 1.
    public mutating func next() -> UInt8? {
        if self.index < self.length {
            let rawOffset: lang.i64 = self.index.raw;
            let bytePtr: lang.ptr[lang.i8] = lang.ptr_offset[lang.i8](self.ptr, rawOffset);
            let signedByte: lang.i8 = unsafe lang.ptr_read(bytePtr);
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
/// A `(slice, length)` pair: the borrowed `StringSlice` and its byte count.
///
/// # Memory Model
///
/// Borrows the source `String` and reads its current buffer on every
/// access, so it can't outlive the string. Copy out to a new `String`
/// (e.g. via `substring`) if you need an independent value.
public struct BytesView: Iterable, Cloneable {
    /// The element type yielded by iteration — always `UInt8`.
    type Item = UInt8
    /// The iterator type returned by `iter()`.
    type TargetIterator = BytesIterator

    let slice: &StringSlice
    // Re-read on every access: the string may have changed since.
    fileprivate var ptr: lang.ptr[lang.i8] { self.slice._bytes() }
    // The slice's byte count.
    fileprivate var length: Int64 { self.slice._len }

    public func clone() -> BytesView { BytesView(slice: self.slice) }

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
    /// boundaries — call `.toOwned()` on the sub-view if you need an
    /// owned `String` (which validates).
    public subscript[I](index: I) -> I.Output where I: ViewIndex[BytesView] {
        get { I.read(index, self) }
    }

    /// @name Checked Index
    /// Reads at `index`, returning `.None` on out-of-bounds.
    public subscript[I](checked index: I) -> I.Output? where I: ViewIndex[BytesView] {
        get { I.readChecked(index, self) }
    }

    /// @name Unchecked Index
    /// Reads at `index` with no bounds check.
    ///
    /// # Safety
    ///
    /// Caller must guarantee `0 <= index < count`. For ranges, the
    /// endpoints must be in `0..=count`; otherwise the resulting
    /// sub-view aliases out-of-bounds memory.
    public unsafe subscript[I](unchecked index: I) -> I.Output where I: ViewIndex[BytesView] {
        get { unsafe I.readUnchecked(index, self) }
    }

    /// @name Clamping
    /// The byte at `index` saturated into `[0, count)`, or `.None` when
    /// the view is empty.
    public subscript(clamped index: Int64) -> UInt8? {
        get { unitClamped(self, index) }
    }

    /// @name Within
    /// The sub-view of `range` clamped into `[0, count)` (always valid,
    /// possibly empty).
    public subscript[R](within range: R) -> BytesView where R: SeqRange {
        get { sliceWithin(self, range) }
    }

    /// @name Wrapping
    /// Reads at `index` with modulo wrap-around. Negative indices wrap
    /// from the end: `view.bytes(wrapped: -1)` reads the last byte.
    /// Returns `None` on an empty view.
    public subscript[I](wrapped index: I) -> I.WrappedOutput where I: ViewWrappable[BytesView] {
        get { I.readWrapped(index, self) }
    }

    /// Returns a `BytesIterator` positioned at byte 0.
    ///
    /// Required by `Iterable`. Each call produces a fresh iterator —
    /// the view is reusable.
    public func iter() -> BytesIterator {
        BytesIterator(source: self.slice, length: self.length, index: 0)
    }

    /// Internal: read a single byte at `index` without bounds check.
    /// Used by the subscript conformances and substring helpers.
    fileprivate func _readByteRaw(index index: Int64) -> UInt8 {
        let rawOffset: lang.i64 = index.raw;
        let bytePtr: lang.ptr[lang.i8] = lang.ptr_offset[lang.i8](self.ptr, rawOffset);
        let signedByte: lang.i8 = unsafe lang.ptr_read(bytePtr);
        UInt8(raw: signedByte)
    }

    /// Materializes the view as an owned `String`. Copies all bytes
    /// into a fresh buffer; the result is independent of the source.
    /// The bytes are validated as UTF-8: any invalid sequence (e.g.
    /// from a sub-view cut mid-codepoint) is replaced with U+FFFD,
    /// so the result is always a well-formed `String`.
    public func toOwned() -> String {
        String(fromUtf8Lossy: unsafe UnsafeSlice(
            pointer: self.slice._rawPtr(),
            count: self.length
        ).asSlice())
    }

    /// The byte range as an owned `String`: `self(range).toOwned()`
    /// for any range form. Panics when the range doesn't fit.
    public func substring[R](range: R) -> String where R: SeqRange {
        sliceOf(self, range).toOwned()
    }

    /// Internal: build a sub-view over byte range `[startByte, endByte)`.
    fileprivate func _subView(startByte startByte: Int64, endByte endByte: Int64) -> BytesView {
        BytesView(slice: self.slice.subslice(from: startByte, to: endByte))
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
/// A `(source, length, byteIndex)` triple. `byteIndex` walks the buffer
/// in variable-width steps according to the UTF-8 encoding.
///
/// # Memory Model
///
/// Borrows the source `String` and reads its current buffer on every
/// `next()`, so it can't outlive the string and never reads freed memory.
public struct CharsIterator: Iterator {
    /// The element type yielded by `next()` — always `Char`.
    type Item = Char

    let source: &StringSlice
    // Re-read on every access: the string may have changed since.
    private var ptr: lang.ptr[lang.i8] { self.source._bytes() }
    var length: Int64
    var byteIndex: Int64

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
/// `CharsView` sub-view — call `.toOwned()` to materialize an owned
/// `String`. To index in O(1), use `BytesView` and convert byte offsets
/// back yourself.
///
/// # Examples
///
/// ```
/// let s = "héllo";
/// s.chars.count;                       // 5 (code points)
/// s.bytes.count;                       // 6 (bytes — 'é' is 2 bytes)
/// s.chars(0..<4).toOwned();           // "héll"
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

    let slice: &StringSlice
    // Re-read on every access: the string may have changed since.
    fileprivate var ptr: lang.ptr[lang.i8] { self.slice._bytes() }
    // The slice's byte count.
    fileprivate var length: Int64 { self.slice._len }

    public func clone() -> CharsView { CharsView(slice: self.slice) }

    /// Returns a `CharsIterator` positioned at byte 0.
    ///
    /// Each call returns a fresh iterator; the view itself is reusable.
    public func iter() -> CharsIterator {
        CharsIterator(source: self.slice, length: self.length, byteIndex: 0)
    }

    /// `true` when the view spans zero bytes (no code points).
    ///
    /// O(1) — checks the byte length, not `count`.
    public var isEmpty: Bool { self.length == 0 }

    /// Number of code points. **O(n)** — walks the buffer counting
    /// UTF-8 leading bytes (those whose top two bits are not `10`). For
    /// ASCII strings this equals `bytes.count`. Cache the result if you
    /// need it more than once; each access re-walks the string.
    // TODO: replace lang.i32_*/lang.cast_i8_i32/lang.ptr_* intrinsics in
    // byte-classification code with UInt8/RawPointer wrappers after LLVM switch
    public var count: Int64 {
        var n: Int64 = 0;
        for i in 0..<self.length {
            // Count leading bytes only (not continuation bytes 10xxxxxx)
            let rawOffset: lang.i64 = i.raw;
            let bytePtr: lang.ptr[lang.i8] = lang.ptr_offset[lang.i8](self.ptr, rawOffset);
            let signedByte: lang.i8 = unsafe lang.ptr_read(bytePtr);
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
    public subscript[I](index: I) -> I.Output where I: ViewIndex[CharsView] {
        get { I.read(index, self) }
    }

    /// @name Checked Index
    /// Reads at `index`, returning `.None` on out-of-bounds.
    public subscript[I](checked index: I) -> I.Output? where I: ViewIndex[CharsView] {
        get { I.readChecked(index, self) }
    }

    /// @name Clamping
    /// The char at `index` saturated into `[0, count)`, or `.None` when
    /// the view is empty.
    public subscript(clamped index: Int64) -> Char? {
        get { unitClamped(self, index) }
    }

    /// @name Within
    /// The sub-view of `range` clamped into `[0, count)` (always valid,
    /// possibly empty).
    public subscript[R](within range: R) -> CharsView where R: SeqRange {
        get { sliceWithin(self, range) }
    }

    /// @name Wrapping
    /// Reads at `index` with modulo wrap-around. Negative indices wrap
    /// from the end: `view.chars(wrapped: -1)` reads the last char.
    /// Returns `None` on an empty view.
    public subscript[I](wrapped index: I) -> I.WrappedOutput where I: ViewWrappable[CharsView] {
        get { I.readWrapped(index, self) }
    }

    /// Internal: the code point starting at byte `byteOffset` (U+FFFD for an
    /// invalid sequence), or `.None` outside the buffer.
    fileprivate func _charAtByte(byteOffset: Int64) -> Char? {
        if byteOffset < 0 or byteOffset >= self.length {
            return .None
        }
        match decodeUtf8(self.ptr, self.length, at: byteOffset) {
            .Some(d) => .Some(d.char),
            .None => .Some(Char(unchecked: 0xFFFD))
        }
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
        CharsView(slice: self.slice.subslice(from: startByte, to: endByte))
    }

    /// Materializes the view as an owned `String`. O(n) — copies bytes.
    public func toOwned() -> String {
        _copyByteRange(ptr: self.ptr, startByte: 0, endByte: self.length)
    }

    /// The code-point range as an owned `String`: `self(range).toOwned()`
    /// for any range form. Panics when the range doesn't fit.
    public func substring[R](range: R) -> String where R: SeqRange {
        sliceOf(self, range).toOwned()
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
            .Some(Grapheme(char: unsafe chars(unchecked: 0)))
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

    let slice: &StringSlice
    // Re-read on every access: the string may have changed since.
    fileprivate var ptr: lang.ptr[lang.i8] { self.slice._bytes() }
    // The slice's byte count.
    fileprivate var length: Int64 { self.slice._len }

    public func clone() -> GraphemesView { GraphemesView(slice: self.slice) }

    /// Returns a `GraphemesIterator` positioned at byte 0.
    public func iter() -> GraphemesIterator {
        GraphemesIterator(CharsIterator(source: self.slice, length: self.length, byteIndex: 0))
    }

    /// `true` when the view spans zero bytes (no graphemes).
    ///
    /// O(1) — checks the byte length, not `count`.
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
    public subscript[I](index: I) -> I.Output where I: ViewIndex[GraphemesView] {
        get { I.read(index, self) }
    }

    /// @name Checked Index
    /// Reads at `index`, returning `.None` on out-of-bounds.
    public subscript[I](checked index: I) -> I.Output? where I: ViewIndex[GraphemesView] {
        get { I.readChecked(index, self) }
    }

    /// @name Clamping
    /// The grapheme at `index` saturated into `[0, count)`, or `.None` when
    /// the view is empty.
    public subscript(clamped index: Int64) -> Grapheme? {
        get { unitClamped(self, index) }
    }

    /// @name Within
    /// The sub-view of `range` clamped into `[0, count)` (always valid,
    /// possibly empty).
    public subscript[R](within range: R) -> GraphemesView where R: SeqRange {
        get { sliceWithin(self, range) }
    }

    /// @name Wrapping
    /// Reads at `index` with modulo wrap-around. Negative indices wrap
    /// from the end: `view.graphemes(wrapped: -1)` reads the last
    /// grapheme cluster. Returns `None` on an empty view.
    public subscript[I](wrapped index: I) -> I.WrappedOutput where I: ViewWrappable[GraphemesView] {
        get { I.readWrapped(index, self) }
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
        GraphemesView(slice: self.slice.subslice(from: startByte, to: endByte))
    }

    /// Materializes the view as an owned `String`. O(n) — copies bytes.
    public func toOwned() -> String {
        _copyByteRange(ptr: self.ptr, startByte: 0, endByte: self.length)
    }

    /// The cluster range as an owned `String`: `self(range).toOwned()`
    /// for any range form. Panics when the range doesn't fit.
    public func substring[R](range: R) -> String where R: SeqRange {
        sliceOf(self, range).toOwned()
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
    /// The element type yielded by `next()`: a zero-copy slice of the
    /// source. Call `toOwned()` to keep a line as a `String`.
    type Item = &StringSlice

    let source: &StringSlice
    // Re-read on every access: the string may have changed since.
    private var ptr: lang.ptr[lang.i8] { self.source._bytes() }
    fileprivate var length: Int64
    fileprivate var byteIndex: Int64
    fileprivate var done: Bool

    /// Returns the next line, or `None` once exhausted.
    ///
    /// Scans byte-by-byte for `\n` or `\r`, treating `\r\n` as a
    /// single terminator. The yielded string contains the bytes up
    /// to but not including the terminator.
    public mutating func next() -> (&StringSlice)? {
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
            let byte: lang.i8 = unsafe lang.ptr_read(bytePtr);
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
                    let nextByte: lang.i8 = unsafe lang.ptr_read(nextBytePtr);
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

        let none: (&StringSlice)? = .None;
        none
    }

    /// Collects the remaining lines as owned strings. (A line borrows the
    /// source string, so an array of slices can't exist.)
    public consuming func collect() -> Array[String] {
        var result = Array[String]();
        while let .Some(line) = self.next() {
            result.append(line.toOwned())
        }
        result
    }

    /// Bytes `[start, end)` of the source as a slice sharing its storage.
    private func createSubstring(start: Int64, end: Int64) -> &StringSlice {
        self.source.subslice(from: start, to: end)
    }
}

/// A view over the lines of a `String`, split on `\n`, `\r\n`, or `\r`.
///
/// Returned by `String.lines`. Iteration yields zero-copy `StringSlice`s and
/// single-line subscripting yields a `String`; neither includes the terminator; a trailing
/// line without a terminator is still emitted. Range subscripts
/// (`lines(0..<n)`) yield a zero-copy `LinesView` sub-view whose
/// underlying byte range still includes the original terminators —
/// iterating the sub-view round-trips the same line strings, and
/// `.toOwned()` reconstructs the original substring exactly.
///
/// # Examples
///
/// ```
/// var lines = Array[String]();
/// for line in "a\nb\nc".lines {
///     lines.append(line.toOwned());
/// }
/// lines.count;  // 3
///
/// // Range subscript preserves terminators in the underlying bytes:
/// "a\r\nb\nc".lines(0..<2).toOwned();  // "a\r\nb\n"
/// ```
///
/// # Representation
///
/// A `(ptr, length)` pair pointing into the source string.
public struct LinesView: Iterable, Cloneable {
    /// The element type yielded by iteration: a zero-copy `StringSlice`.
    type Item = &StringSlice
    /// The iterator type returned by `iter()`.
    type TargetIterator = LinesIterator

    let slice: &StringSlice
    // Re-read on every access: the string may have changed since.
    fileprivate var ptr: lang.ptr[lang.i8] { self.slice._bytes() }
    // The slice's byte count.
    fileprivate var length: Int64 { self.slice._len }

    public func clone() -> LinesView { LinesView(slice: self.slice) }

    /// Returns a `LinesIterator` positioned at byte 0.
    public func iter() -> LinesIterator {
        LinesIterator(source: self.slice, length: self.length, byteIndex: 0, done: false)
    }

    /// `true` when the view spans zero bytes (no lines).
    ///
    /// O(1) — checks the byte length, not `count`.
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
    public subscript[I](index: I) -> I.Output where I: ViewIndex[LinesView] {
        get { I.read(index, self) }
    }

    /// @name Checked Index
    /// Reads at `index`, returning `.None` on out-of-bounds.
    public subscript[I](checked index: I) -> I.Output? where I: ViewIndex[LinesView] {
        get { I.readChecked(index, self) }
    }

    /// @name Clamping
    /// The line at `index` saturated into `[0, count)`, or `.None` when
    /// the view is empty.
    public subscript(clamped index: Int64) -> String? {
        get { unitClamped(self, index) }
    }

    /// @name Within
    /// The sub-view of `range` clamped into `[0, count)` (always valid,
    /// possibly empty).
    public subscript[R](within range: R) -> LinesView where R: SeqRange {
        get { sliceWithin(self, range) }
    }

    /// @name Wrapping
    /// Reads at `index` with modulo wrap-around. Negative indices wrap
    /// from the end: `view.lines(wrapped: -1)` reads the last line.
    /// Returns `None` on an empty view.
    public subscript[I](wrapped index: I) -> I.WrappedOutput where I: ViewWrappable[LinesView] {
        get { I.readWrapped(index, self) }
    }

    /// Materializes the view as an owned `String` covering the entire
    /// underlying buffer (terminators included). O(n) — copies bytes.
    public func toOwned() -> String {
        _copyByteRange(ptr: self.ptr, startByte: 0, endByte: self.length)
    }

    /// The line range as an owned `String`, terminators included: `self(range).toOwned()`
    /// for any range form. Panics when the range doesn't fit.
    public func substring[R](range: R) -> String where R: SeqRange {
        sliceOf(self, range).toOwned()
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
            let byte: lang.i8 = unsafe lang.ptr_read(bytePtr);
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
                    let nextByte: lang.i8 = unsafe lang.ptr_read(nextBytePtr);
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
        LinesView(slice: self.slice.subslice(from: startByte, to: endByte))
    }
}

// ============================================================================
// ASCII VIEW
// ============================================================================

/// Single-pass forward iterator over an ASCII string's characters.
///
/// Yielded by `AsciiView.iter()`. Each byte is one `Char`, so the walk is
/// byte by byte with no decoding.
///
/// # Examples
///
/// ```
/// var it = "hi".ascii!.iter();
/// it.next();  // Some('h')
/// it.next();  // Some('i')
/// it.next();  // None
/// ```
///
/// # Memory Model
///
/// Borrows the source `String` and reads its current buffer on every
/// `next()`, so it can't outlive the string and never reads freed memory.
public struct AsciiIterator: Iterator {
    /// The element type yielded by `next()` — always `Char`.
    type Item = Char

    let source: &StringSlice
    // Re-read on every access: the string may have changed since.
    private var ptr: lang.ptr[lang.i8] { self.source._bytes() }
    fileprivate var length: Int64
    fileprivate var index: Int64

    /// Returns the next character, or `None` at the end.
    public mutating func next() -> Char? {
        if self.index >= self.length {
            return .None
        }
        let byte = asciiCharAt(self.ptr, self.index);
        self.index = self.index + 1;
        .Some(byte)
    }
}

/// The ASCII byte at `index` as a `Char`.
fileprivate func asciiCharAt(ptr: lang.ptr[lang.i8], index: Int64) -> Char {
    let signedByte: lang.i8 = unsafe lang.ptr_read(lang.ptr_offset[lang.i8](ptr, index.raw));
    Char(unchecked: UInt32(from: UInt8(raw: signedByte)))
}

/// A read-only view over an ASCII string, indexed by character in O(1).
///
/// Returned by `s.ascii`, which checks the string once and gives `.None`
/// when any byte is non-ASCII. Every byte is then one `Char`, so `view(i)`,
/// `count` and range slicing cost the same as on `BytesView` while reading
/// as characters: `view(i) == ','` rather than `bytes(i) == 0x2C`.
///
/// # Examples
///
/// ```
/// guard let .Some(text) = "a,b".ascii else { return }
/// text(1);              // ','
/// text(2..).toOwned();  // "b"
/// text.count;           // 3
/// ```
///
/// # Memory Model
///
/// Borrows the source `String` and reads its current buffer on every
/// access, so it can't outlive the string. Copy out (`toOwned`,
/// `substring`) for an independent value.
public struct AsciiView: Iterable, Cloneable {
    /// The element type yielded by iteration — always `Char`.
    type Item = Char
    /// The iterator type returned by `iter()`.
    type TargetIterator = AsciiIterator

    let slice: &StringSlice
    // Re-read on every access: the string may have changed since.
    fileprivate var ptr: lang.ptr[lang.i8] { self.slice._bytes() }
    // The slice's byte count.
    fileprivate var length: Int64 { self.slice._len }

    public func clone() -> AsciiView { AsciiView(slice: self.slice) }

    /// Number of characters, which is the byte count. O(1).
    public var count: Int64 { self.length }

    /// `true` if the view is empty.
    public var isEmpty: Bool { self.length == 0 }

    /// @name Indexed Char / Sub-view
    /// The character at an `Int64` index, or a zero-copy `AsciiView` for a
    /// range. O(1). Panics on out-of-bounds.
    public subscript[I](index: I) -> I.Output where I: ViewIndex[AsciiView] {
        get { I.read(index, self) }
    }

    /// @name Checked Index
    /// Reads at `index`, returning `.None` on out-of-bounds.
    public subscript[I](checked index: I) -> I.Output? where I: ViewIndex[AsciiView] {
        get { I.readChecked(index, self) }
    }

    /// @name Unchecked Index
    /// Reads at `index` with no bounds check.
    ///
    /// # Safety
    ///
    /// Caller must guarantee `0 <= index < count`; for ranges, endpoints in
    /// `0..=count`.
    public unsafe subscript[I](unchecked index: I) -> I.Output where I: ViewIndex[AsciiView] {
        get { unsafe I.readUnchecked(index, self) }
    }

    /// @name Clamping
    /// The character at `index` saturated into `[0, count)`, or `.None`
    /// when the view is empty.
    public subscript(clamped index: Int64) -> Char? {
        get { unitClamped(self, index) }
    }

    /// @name Within
    /// The sub-view of `range` clamped into `[0, count)` (always valid,
    /// possibly empty).
    public subscript[R](within range: R) -> AsciiView where R: SeqRange {
        get { sliceWithin(self, range) }
    }

    /// @name Wrapping
    /// Reads at `index` with modulo wrap-around: `view(wrapped: -1)` is the
    /// last character. `None` on an empty view.
    public subscript[I](wrapped index: I) -> I.WrappedOutput where I: ViewWrappable[AsciiView] {
        get { I.readWrapped(index, self) }
    }

    /// Returns an `AsciiIterator` positioned at the start.
    public func iter() -> AsciiIterator {
        AsciiIterator(source: self.slice, length: self.length, index: 0)
    }

    /// The view as an owned `String`.
    public func toOwned() -> String {
        _copyByteRange(ptr: self.ptr, startByte: 0, endByte: self.length)
    }

    /// The character range as an owned `String`: `self(range).toOwned()`
    /// for any range form. Panics when the range doesn't fit.
    public func substring[R](range: R) -> String where R: SeqRange {
        sliceOf(self, range).toOwned()
    }

    /// Internal: build a sub-view over byte range `[startByte, endByte)`.
    fileprivate func _subView(startByte startByte: Int64, endByte endByte: Int64) -> AsciiView {
        AsciiView(slice: self.slice.subslice(from: startByte, to: endByte))
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

/// Prints the wrapped byte offset, honoring width and alignment like an
/// `Int64` (`"\(i)"` is `"3"`).
extend ByteIndex: Formattable {
    public func format(mutating into writer: some Formatter, options: FormatOptions = FormatOptions.default()) {
        self.value.format(into: writer, options)
    }
}

/// Prints the byte offset where the character starts.
extend CharIndex: Formattable {
    public func format(mutating into writer: some Formatter, options: FormatOptions = FormatOptions.default()) {
        self.byteOffset.format(into: writer, options)
    }
}

/// Prints the byte offset where the grapheme starts.
extend GraphemeIndex: Formattable {
    public func format(mutating into writer: some Formatter, options: FormatOptions = FormatOptions.default()) {
        self.byteOffset.format(into: writer, options)
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
// SHARED INDEXING
// ============================================================================
//
// Every indexable view answers the same five questions (count, the unit at a
// position, a unit position's byte offset, a sub-view over bytes); the bounds
// rules for each access mode live once, below, over any `TextView`.

/// A view that indexes a string by some unit (byte, char, grapheme, line).
internal protocol TextView {
    /// What an `Int64` index reads: one unit.
    type Element

    /// The number of units.
    var count: Int64 { get }

    /// The unit at `index`, or `.None` outside `[0, count)`.
    func _unitAt(index: Int64) -> Element?

    /// The unit at `index`, which the caller guarantees is in range.
    unsafe func _unitUnchecked(index: Int64) -> Element

    /// The byte offset where unit `unit` starts (`count` gives the byte
    /// length), or `.None` outside `[0, count]`.
    func _byteOffset(unit: Int64) -> Int64?

    /// The sub-view over bytes `[startByte, endByte)`.
    func _subView(startByte startByte: Int64, endByte endByte: Int64) -> Self
}

/// The unit at `index`, trapping when it's out of range.
fileprivate func unitAt[V](view: V, index: Int64) -> V.Element where V: TextView, V: not Static {
    guard let .Some(unit) = view._unitAt(index) else {
        fatalError("text view index out of bounds")
    }
    unit
}

/// `view(clamped: index)`: the unit at `index` saturated into `[0, count)`,
/// or `.None` when the view is empty.
fileprivate func unitClamped[V](view: V, index: Int64) -> V.Element? where V: TextView, V: not Static {
    let n = view.count;
    if n == 0 {
        return .None
    }
    var i = index;
    if i < 0 { i = 0 }
    if i >= n { i = n - 1 }
    view._unitAt(i)
}

/// `view(wrapped: index)`: the unit at `index` modulo `count`, so `-1` is
/// the last; `.None` when the view is empty.
fileprivate func unitWrapped[V](view: V, index: Int64) -> V.Element? where V: TextView, V: not Static {
    let n = view.count;
    if n == 0 {
        return .None
    }
    view._unitAt(wrapIndex(index, n))
}

/// `index` modulo `n`, in `[0, n)`.
fileprivate func wrapIndex(index: Int64, n: Int64) -> Int64 {
    let i = index % n;
    if i < 0 { i + n } else { i }
}

/// A range's start as a unit position.
fileprivate func startUnit(bound: RangeBound) -> Int64 {
    match bound {
        .Included(i) => i,
        .Excluded(i) => i + 1,
        .Unbounded => 0
    }
}

/// A range's end as an exclusive unit position; `.None` runs to the end.
fileprivate func endUnit(bound: RangeBound) -> Int64? {
    match bound {
        .Included(i) => .Some(i + 1),
        .Excluded(i) => .Some(i),
        .Unbounded => .None
    }
}

/// The sub-view over units `[start, end)`; `end` of `.None` runs to the end.
/// `.None` unless `0 <= start <= end <= count`.
fileprivate func sliceChecked[V, R](view: V, range: R) -> V? where V: TextView, V: not Static, R: SeqRange {
    let start = startUnit(range.startBound);
    let end = endUnit(range.endBound);
    if start < 0 {
        return .None
    }
    if let .Some(e) = end {
        if start > e {
            return .None
        }
    }
    guard let .Some(startByte) = view._byteOffset(start) else {
        return .None
    }
    guard let .Some(e) = end else {
        return .Some(view._subView(startByte: startByte, endByte: view._byteOffset(view.count) ?? startByte))
    }
    guard let .Some(endByte) = view._byteOffset(e) else {
        return .None
    }
    .Some(view._subView(startByte: startByte, endByte: endByte))
}

/// `view(range)`: the sub-view, trapping when the range doesn't fit.
fileprivate func sliceOf[V, R](view: V, range: R) -> V where V: TextView, V: not Static, R: SeqRange {
    guard let .Some(sub) = sliceChecked(view, range) else {
        fatalError("text view range out of bounds")
    }
    sub
}

/// The sub-view over units `[start, end)`, which must already fit.
fileprivate func sliceUnits[V](view: V, start: Int64, end: Int64) -> V where V: TextView, V: not Static {
    let startByte = view._byteOffset(start) ?? 0;
    let endByte = view._byteOffset(end) ?? startByte;
    view._subView(startByte: startByte, endByte: endByte)
}

/// `view(unchecked: range)`: the sub-view with no bounds check; an open
/// end runs to the end.
fileprivate func sliceUnchecked[V, R](view: V, range: R) -> V where V: TextView, V: not Static, R: SeqRange {
    let start = startUnit(range.startBound);
    sliceUnits(view, start, endUnit(range.endBound) ?? view.count)
}

/// `view(within: range)`: the range clamped into `[0, count]`; always
/// valid, possibly empty.
fileprivate func sliceWithin[V, R](view: V, range: R) -> V where V: TextView, V: not Static, R: SeqRange {
    let n = view.count;
    let start = clampUnit(startUnit(range.startBound), n);
    let end = clampUnit(endUnit(range.endBound) ?? n, n);
    sliceUnits(view, if start > end { end } else { start }, end)
}

/// `position` saturated into `[0, n]`.
fileprivate func clampUnit(position: Int64, n: Int64) -> Int64 {
    if position < 0 {
        0
    } else if position > n {
        n
    } else {
        position
    }
}

/// `view(wrapped: range)`: the anchored end wraps modulo `count` and the
/// range keeps its length, cut at the end. A range with a start wraps the
/// start (`-2..` is the last two); one without wraps the end (`..=-1` is
/// everything). Empty on an empty view.
fileprivate func sliceWrapped[V, R](view: V, range: R) -> V where V: TextView, V: not Static, R: SeqRange {
    let n = view.count;
    if n == 0 {
        return view._subView(startByte: 0, endByte: 0)
    }
    let end = endUnit(range.endBound);
    if let .Unbounded = range.startBound {
        // `..<e` wraps `e`; `..=e` wraps `e` and keeps it.
        let wrapped = match range.endBound {
            .Included(i) => wrapIndex(i, n) + 1,
            .Excluded(i) => wrapIndex(i, n),
            .Unbounded => n
        };
        return sliceUnits(view, 0, wrapped)
    }
    let first = startUnit(range.startBound);
    let start = wrapIndex(first, n);
    guard let .Some(e) = end else {
        return sliceUnits(view, start, n)
    }
    var span = e - first;
    if span < 0 { span = 0 }
    sliceUnits(view, start, if start + span > n { n } else { start + span })
}

extend BytesView: TextView {
    type Element = UInt8

    func _unitAt(index: Int64) -> UInt8? {
        if index < 0 or index >= self.length {
            return .None
        }
        .Some(self._readByteRaw(index: index))
    }

    unsafe func _unitUnchecked(index: Int64) -> UInt8 { self._readByteRaw(index: index) }

    func _byteOffset(unit: Int64) -> Int64? {
        if unit < 0 or unit > self.length { .None } else { .Some(unit) }
    }
}

extend CharsView: TextView {
    type Element = Char

    // Walking views have no cheaper path: check anyway.
    unsafe func _unitUnchecked(index: Int64) -> Char { unitAt(self, index) }

    func _unitAt(index: Int64) -> Char? { _charsViewCharAt(view: self, charIndex: index) }

    func _byteOffset(unit: Int64) -> Int64? {
        let (offset, found) = self._byteOffsetForCharIndex(charIndex: unit);
        if found { .Some(offset) } else { .None }
    }
}

extend GraphemesView: TextView {
    type Element = Grapheme

    // Walking views have no cheaper path: check anyway.
    unsafe func _unitUnchecked(index: Int64) -> Grapheme { unitAt(self, index) }

    func _unitAt(index: Int64) -> Grapheme? { _graphemesViewAt(view: self, graphemeIndex: index) }

    func _byteOffset(unit: Int64) -> Int64? {
        let (offset, found) = self._byteOffsetForGraphemeIndex(graphemeIndex: unit);
        if found { .Some(offset) } else { .None }
    }
}

extend AsciiView: TextView {
    type Element = Char

    func _unitAt(index: Int64) -> Char? {
        if index < 0 or index >= self.length {
            return .None
        }
        .Some(asciiCharAt(self.ptr, index))
    }

    unsafe func _unitUnchecked(index: Int64) -> Char { asciiCharAt(self.ptr, index) }

    func _byteOffset(unit: Int64) -> Int64? {
        if unit < 0 or unit > self.length { .None } else { .Some(unit) }
    }
}

extend LinesView: TextView {
    type Element = String

    // Walking views have no cheaper path: check anyway.
    unsafe func _unitUnchecked(index: Int64) -> String { unitAt(self, index) }

    func _unitAt(index: Int64) -> String? { _linesViewAt(view: self, lineIndex: index) }

    func _byteOffset(unit: Int64) -> Int64? {
        let (offset, found) = self._byteOffsetForLineIndex(lineIndex: unit);
        if found { .Some(offset) } else { .None }
    }
}

// ============================================================================
// UNIT LOOKUP
// ============================================================================

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
                return .Some(line.toOwned())
            }
            li = li + 1
        } else {
            return .None
        }
    }
    .None
}

// ============================================================================
// VIEW INDEXES
// ============================================================================

/// What a text view's subscript accepts: `Int64` reads one unit, a range
/// a zero-copy sub-view, and a typed index (`ByteIndex`, `CharIndex`) the
/// unit at its byte offset.
internal protocol ViewIndex[V] {
    /// What the index reads.
    type Output

    /// `view(index)`: panics when it doesn't fit.
    @sugarOnly
    static func read(consuming index: Self, view: V) -> Output

    /// `view(checked: index)`: `.None` when it doesn't fit.
    @sugarOnly
    static func readChecked(consuming index: Self, view: V) -> Output?

    /// `view(unchecked: index)`: no bounds check.
    @sugarOnly
    static unsafe func readUnchecked(consuming index: Self, view: V) -> Output
}

/// What `view(wrapped:)` accepts: an `Int64` or a range, wrapped modulo
/// the view's count.
internal protocol ViewWrappable[V] {
    /// What the wrapped index reads: an optional unit, or a sub-view.
    type WrappedOutput

    @sugarOnly
    static func readWrapped(consuming index: Self, view: V) -> WrappedOutput
}

extend Int64: ViewIndex[V] where V: TextView {
    type Output = V.Element

    public static func read(consuming index: Int64, view: V) -> V.Element { unitAt(view, index) }

    public static func readChecked(consuming index: Int64, view: V) -> V.Element? { view._unitAt(index) }

    public static unsafe func readUnchecked(consuming index: Int64, view: V) -> V.Element {
        unsafe view._unitUnchecked(index)
    }
}

extend Int64: ViewWrappable[V] where V: TextView {
    type WrappedOutput = V.Element?

    public static func readWrapped(consuming index: Int64, view: V) -> V.Element? { unitWrapped(view, index) }
}

extend Range[Int64]: ViewIndex[V] where V: TextView {
    type Output = V
    public static func read(consuming index: Range[Int64], view: V) -> V { sliceOf(view, index) }
    public static func readChecked(consuming index: Range[Int64], view: V) -> V? { sliceChecked(view, index) }
    public static unsafe func readUnchecked(consuming index: Range[Int64], view: V) -> V { sliceUnchecked(view, index) }
}

extend Range[Int64]: ViewWrappable[V] where V: TextView {
    type WrappedOutput = V
    public static func readWrapped(consuming index: Range[Int64], view: V) -> V { sliceWrapped(view, index) }
}

extend ClosedRange[Int64]: ViewIndex[V] where V: TextView {
    type Output = V
    public static func read(consuming index: ClosedRange[Int64], view: V) -> V { sliceOf(view, index) }
    public static func readChecked(consuming index: ClosedRange[Int64], view: V) -> V? { sliceChecked(view, index) }
    public static unsafe func readUnchecked(consuming index: ClosedRange[Int64], view: V) -> V { sliceUnchecked(view, index) }
}

extend ClosedRange[Int64]: ViewWrappable[V] where V: TextView {
    type WrappedOutput = V
    public static func readWrapped(consuming index: ClosedRange[Int64], view: V) -> V { sliceWrapped(view, index) }
}

extend RangeFrom[Int64]: ViewIndex[V] where V: TextView {
    type Output = V
    public static func read(consuming index: RangeFrom[Int64], view: V) -> V { sliceOf(view, index) }
    public static func readChecked(consuming index: RangeFrom[Int64], view: V) -> V? { sliceChecked(view, index) }
    public static unsafe func readUnchecked(consuming index: RangeFrom[Int64], view: V) -> V { sliceUnchecked(view, index) }
}

extend RangeFrom[Int64]: ViewWrappable[V] where V: TextView {
    type WrappedOutput = V
    public static func readWrapped(consuming index: RangeFrom[Int64], view: V) -> V { sliceWrapped(view, index) }
}

extend RangeUpTo[Int64]: ViewIndex[V] where V: TextView {
    type Output = V
    public static func read(consuming index: RangeUpTo[Int64], view: V) -> V { sliceOf(view, index) }
    public static func readChecked(consuming index: RangeUpTo[Int64], view: V) -> V? { sliceChecked(view, index) }
    public static unsafe func readUnchecked(consuming index: RangeUpTo[Int64], view: V) -> V { sliceUnchecked(view, index) }
}

extend RangeUpTo[Int64]: ViewWrappable[V] where V: TextView {
    type WrappedOutput = V
    public static func readWrapped(consuming index: RangeUpTo[Int64], view: V) -> V { sliceWrapped(view, index) }
}

extend RangeThrough[Int64]: ViewIndex[V] where V: TextView {
    type Output = V
    public static func read(consuming index: RangeThrough[Int64], view: V) -> V { sliceOf(view, index) }
    public static func readChecked(consuming index: RangeThrough[Int64], view: V) -> V? { sliceChecked(view, index) }
    public static unsafe func readUnchecked(consuming index: RangeThrough[Int64], view: V) -> V { sliceUnchecked(view, index) }
}

extend RangeThrough[Int64]: ViewWrappable[V] where V: TextView {
    type WrappedOutput = V
    public static func readWrapped(consuming index: RangeThrough[Int64], view: V) -> V { sliceWrapped(view, index) }
}

extend ByteIndex: ViewIndex[BytesView] {
    type Output = UInt8

    public static func read(consuming index: ByteIndex, view: BytesView) -> UInt8 { unitAt(view, index.value) }

    public static func readChecked(consuming index: ByteIndex, view: BytesView) -> UInt8? { view._unitAt(index.value) }

    public static unsafe func readUnchecked(consuming index: ByteIndex, view: BytesView) -> UInt8 {
        view._readByteRaw(index: index.value)
    }
}

extend CharIndex: ViewIndex[CharsView] {
    type Output = Char

    public static func read(consuming index: CharIndex, view: CharsView) -> Char {
        guard let .Some(c) = view._charAtByte(index.byteOffset) else {
            fatalError("CharsView index out of bounds")
        }
        c
    }

    public static func readChecked(consuming index: CharIndex, view: CharsView) -> Char? { view._charAtByte(index.byteOffset) }

    public static unsafe func readUnchecked(consuming index: CharIndex, view: CharsView) -> Char {
        view._charAtByte(index.byteOffset) ?? Char(unchecked: 0xFFFD)
    }
}

// -- View range-slice methods ------------------------------------------------

extend BytesView {
    /// Returns a `StringSlice` covering the byte range `[start, end)`.
    public func slice(from start: ByteIndex, to end: ByteIndex) -> &StringSlice {
        self.slice.subslice(from: start.value, to: end.value)
    }

    /// Byte index of the first byte.
    public var startIndex: ByteIndex { ByteIndex(0) }

    /// Byte index one past the last byte.
    public var endIndex: ByteIndex { ByteIndex(self.length) }
}

extend CharsView {
    /// Returns a `StringSlice` covering `[start, end)` by byte offset.
    public func slice(from start: CharIndex, to end: CharIndex) -> &StringSlice {
        self.slice.subslice(from: start.byteOffset, to: end.byteOffset)
    }

    /// Char index at byte 0 (the first code point).
    public var startIndex: CharIndex { CharIndex(0) }

    /// Char index at the end (one past the last byte).
    public var endIndex: CharIndex { CharIndex(self.length) }
}

extend GraphemesView {
    /// Returns a `StringSlice` covering `[start, end)` by byte offset.
    public func slice(from start: GraphemeIndex, to end: GraphemeIndex) -> &StringSlice {
        self.slice.subslice(from: start.byteOffset, to: end.byteOffset)
    }

    /// Grapheme index at byte 0.
    public var startIndex: GraphemeIndex { GraphemeIndex(0) }

    /// Grapheme index at the end (one past the last byte).
    public var endIndex: GraphemeIndex { GraphemeIndex(self.length) }
}

extend LinesView {
    /// Returns a `StringSlice` covering `[start, end)` by byte offset.
    public func slice(from start: LineIndex, to end: LineIndex) -> &StringSlice {
        self.slice.subslice(from: start.byteOffset, to: end.byteOffset)
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
    public func advance(by n: Int64, from source: &StringSlice) -> GraphemeIndex {
        let rawPtr: lang.ptr[lang.i8] = lang.cast_ptr[_, lang.i8](source._rawPtr().asRaw().raw);
        let length = source._len;
        var offset = self.byteOffset;
        var remaining = n;

        var charsIter = CharsIterator(source: source, length: length, byteIndex: offset);
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
    public func advance(by n: Int64, from source: &StringSlice) -> LineIndex {
        let rawPtr: lang.ptr[lang.i8] = lang.cast_ptr[_, lang.i8](source._rawPtr().asRaw().raw);
        let length = source._len;
        var offset = self.byteOffset;
        var remaining = n;

        while remaining > 0 and offset < length {
            let rawOffset: lang.i64 = offset.raw;
            let bytePtr: lang.ptr[lang.i8] = lang.ptr_offset[lang.i8](rawPtr, rawOffset);
            let byte: lang.i8 = unsafe lang.ptr_read(bytePtr);
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
                    let nextByte: lang.i8 = unsafe lang.ptr_read(nextBytePtr);
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

    let source: &StringSlice
    // Re-read on every access: the string may have changed since.
    private var ptr: lang.ptr[lang.i8] { self.source._bytes() }
    fileprivate var length: Int64
    fileprivate var byteIndex: Int64

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

    let source: &StringSlice
    // Re-read on every access: the string may have changed since.
    private var ptr: lang.ptr[lang.i8] { self.source._bytes() }
    fileprivate var length: Int64
    fileprivate var byteIndex: Int64
    fileprivate var done: Bool

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
            let byte: lang.i8 = unsafe lang.ptr_read(bytePtr);
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
                    let nextByte: lang.i8 = unsafe lang.ptr_read(nextBytePtr);
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
        IndexedCharsIterator(source: self.slice, length: self.length, byteIndex: 0)
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
        IndexedLinesIterator(source: self.slice, length: self.length, byteIndex: 0, done: false)
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

    let source: &StringSlice
    // Re-read on every access: the string may have changed since.
    private var ptr: lang.ptr[lang.i8] { self.source._bytes() }
    fileprivate var length: Int64
    fileprivate var byteIndex: Int64

    public mutating func next() -> Char? {
        if self.byteIndex <= 0 { return .None }

        // Walk backwards past continuation bytes (10xxxxxx)
        var i = self.byteIndex - 1;
        while i > 0 {
            let rawOffset: lang.i64 = i.raw;
            let bytePtr: lang.ptr[lang.i8] = lang.ptr_offset[lang.i8](self.ptr, rawOffset);
            let signedByte: lang.i8 = unsafe lang.ptr_read(bytePtr);
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
/// view.first;    // Some('c')
/// view.count;      // 3
/// ```
public struct ReversedCharsView: Iterable, Cloneable {
    type Item = Char
    type TargetIterator = ReversedCharsIterator

    let slice: &StringSlice
    // Re-read on every access: the string may have changed since.
    fileprivate var ptr: lang.ptr[lang.i8] { self.slice._bytes() }
    // The slice's byte count.
    fileprivate var length: Int64 { self.slice._len }

    public func clone() -> ReversedCharsView { ReversedCharsView(slice: self.slice) }

    public func iter() -> ReversedCharsIterator {
        ReversedCharsIterator(source: self.slice, length: self.length, byteIndex: self.length)
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
    public var first: (&StringSlice)? {
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
    type Item = &StringSlice

    let slice: &StringSlice
    let separator: &StringSlice
    // Re-read on every access: the string may have changed since.
    fileprivate var sourcePtr: Pointer[UInt8] { self.slice._rawPtr() }
    fileprivate var sourceLen: Int64 { self.slice._len }
    fileprivate var index: Int64
    fileprivate var done: Bool
    // Pieces still allowed before the rest is yielded unsplit; 0 = no limit.
    fileprivate var piecesLeft: Int64

    public func clone() -> SplitViewIterator {
        SplitViewIterator(slice: self.slice, separator: self.separator, index: self.index, done: self.done, piecesLeft: self.piecesLeft)
    }

    /// Counts a non-final piece against the limit, if there is one.
    fileprivate mutating func tookPiece() {
        if self.piecesLeft > 0 { self.piecesLeft = self.piecesLeft - 1 }
    }

    public mutating func next() -> (&StringSlice)? {
        if self.done { return .None }

        let start = self.index;
        let sepLen = self.separator.bytes.count;

        // Last piece the limit allows: the rest of the source, unsplit. An
        // empty source still has no pieces.
        if self.piecesLeft == 1 {
            self.done = true;
            if self.sourceLen == 0 { return .None }
            return .Some(self.slice.subslice(from: start, to: self.sourceLen))
        }

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
                self.tookPiece();
                return .Some(self.slice.subslice(from: start, to: self.index))
            }
            self.done = true;
            return .None
        }

        // Search for separator via memmem
        let remaining = self.sourceLen - self.index;
        if remaining >= sepLen {
            let base = self.sourcePtr.offset(by: self.index).asRaw();
            let sepSlice = self.separator.asSlice();
            let needle = sepSlice._rawPtr().asRaw();
            let result = unsafe memmem(base, remaining, needle, sepLen);
            if result.isNull == false {
                let diff: lang.i64 = lang.i64_sub(result.address.raw, base.address.raw);
                let matchIndex = self.index + Int64(intLiteral: diff);
                self.index = matchIndex + sepLen;
                self.tookPiece();
                return .Some(self.slice.subslice(from: start, to: matchIndex))
            }
        }

        // No more separators — return the remainder. It is empty when the
        // source ends with a separator (`start > 0` means one was consumed),
        // and that empty segment is kept, just as a leading one is.
        self.done = true;
        if start < self.sourceLen or start > 0 {
            return .Some(self.slice.subslice(from: start, to: self.slice._len))
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
/// let view = "a,b,c".asSlice().split(by: ",");
/// view.first;            // Some("a")
/// view.count;              // 3
/// view.collect();          // [StringSlice("a"), StringSlice("b"), StringSlice("c")]
/// ```
public struct SplitView: Iterable, Cloneable {
    type Item = &StringSlice
    type TargetIterator = SplitViewIterator

    let slice: &StringSlice
    let separator: &StringSlice
    // At most this many pieces, the last holding the rest; 0 = no limit.
    var limit: Int64

    public func clone() -> SplitView {
        SplitView(slice: self.slice, separator: self.separator, limit: self.limit)
    }

    public func iter() -> SplitViewIterator {
        SplitViewIterator(slice: self.slice, separator: self.separator, index: 0, done: false, piecesLeft: self.limit)
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
    public var first: (&StringSlice)? {
        var it = self.iter();
        it.next()
    }

    /// The last segment, or `.None` if empty.
    public var last: (&StringSlice)? {
        var it = self.iter();
        var result: (&StringSlice)? = .None;
        while let .Some(segment) = it.next() {
            result = .Some(segment)
        }
        result
    }

    /// Collects all segments into an array of owned strings. (A segment
    /// borrows the source string, so an array of slices can't exist.)
    public func collect() -> Array[String] {
        var result = Array[String]();
        for segment in self {
            result.append(segment.toOwned())
        }
        result
    }
}

// ============================================================================
// WORDS VIEW
// ============================================================================

/// ASCII whitespace, the same set `trimmed()` strips: space, tab, `\n`,
/// `\r` and form feed.
fileprivate func isWordBreak(b: UInt8) -> Bool {
    b == 32 or b == 9 or b == 10 or b == 13 or b == 12
}

/// Iterator over the whitespace-separated words of a string. Runs of
/// whitespace count as one break, and leading/trailing whitespace yields
/// nothing, so no word is ever empty.
public struct WordsIterator: Iterator, Cloneable {
    type Item = &StringSlice

    let slice: &StringSlice
    fileprivate var index: Int64

    public func clone() -> WordsIterator {
        WordsIterator(slice: self.slice, index: self.index)
    }

    public mutating func next() -> (&StringSlice)? {
        let len = self.slice.bytes.count;
        // Skip the whitespace before the word.
        while self.index < len and isWordBreak(self.slice.bytes(self.index)) {
            self.index = self.index + 1
        }
        if self.index >= len { return .None }
        let start = self.index;
        while self.index < len and isWordBreak(self.slice.bytes(self.index)) == false {
            self.index = self.index + 1
        }
        .Some(self.slice.subslice(from: start, to: self.index))
    }
}

/// Lazy view over the whitespace-separated words of a string, each a
/// zero-copy `StringSlice`. Unlike `split(by: " ")`, runs of whitespace
/// never produce empty pieces.
///
/// # Examples
///
/// ```
/// "  a  b\tc ".words.count;   // 3
/// ```
public struct WordsView: Iterable, Cloneable {
    type Item = &StringSlice
    type TargetIterator = WordsIterator

    let slice: &StringSlice

    public func clone() -> WordsView { WordsView(slice: self.slice) }

    public func iter() -> WordsIterator { WordsIterator(slice: self.slice, index: 0) }

    /// True when there are no words (the source is empty or all whitespace).
    public var isEmpty: Bool {
        var it = self.iter();
        it.next().isNone
    }

    /// Number of words. O(n).
    public var count: Int64 {
        var n: Int64 = 0;
        for _ in self { n = n + 1 }
        n
    }

    /// Collects the words into an array of owned strings. (A word borrows
    /// the source string, so an array of slices can't exist.)
    public func collect() -> Array[String] {
        var result = Array[String]();
        for word in self {
            result.append(word.toOwned())
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
    type Item = &StringSlice

    let slice: &StringSlice
    fileprivate var predicate: escaping (Char) -> Bool
    // Re-read on every access: the string may have changed since.
    fileprivate var sourcePtr: Pointer[UInt8] { self.slice._rawPtr() }
    fileprivate var sourceLen: Int64 { self.slice._len }
    fileprivate var index: Int64
    fileprivate var done: Bool

    public func clone() -> SplitWhereViewIterator {
        SplitWhereViewIterator(slice: self.slice, predicate: self.predicate, index: self.index, done: self.done)
    }

    public mutating func next() -> (&StringSlice)? {
        if self.done { return .None }

        let start = self.index;
        let rawPtr: lang.ptr[lang.i8] = lang.cast_ptr[_, lang.i8](self.sourcePtr.asRaw().raw);

        while self.index < self.sourceLen {
            let result = decodeUtf8(rawPtr, self.sourceLen, at: self.index);
            if let .Some(decoded) = result {
                if self.predicate(decoded.char) {
                    let matchIndex = self.index;
                    self.index = self.index + decoded.bytesConsumed;
                    return .Some(self.slice.subslice(from: start, to: matchIndex))
                }
                self.index = self.index + decoded.bytesConsumed
            } else {
                self.index = self.index + 1
            }
        }

        // No more matches — return the remainder, keeping a trailing empty
        // segment like SplitViewIterator does.
        self.done = true;
        if start < self.sourceLen or start > 0 {
            return .Some(self.slice.subslice(from: start, to: self.slice._len))
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
/// view.first;    // Some("hello")
/// view.count;      // 2
/// ```
public struct SplitWhereView: Iterable, Cloneable {
    type Item = &StringSlice
    type TargetIterator = SplitWhereViewIterator

    let slice: &StringSlice
    var predicate: escaping (Char) -> Bool

    public func clone() -> SplitWhereView {
        SplitWhereView(slice: self.slice, predicate: self.predicate)
    }

    public func iter() -> SplitWhereViewIterator {
        SplitWhereViewIterator(slice: self.slice, predicate: self.predicate, index: 0, done: false)
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
    public var first: (&StringSlice)? {
        var it = self.iter();
        it.next()
    }

    /// The last segment, or `.None` if empty.
    public var last: (&StringSlice)? {
        var it = self.iter();
        var result: (&StringSlice)? = .None;
        while let .Some(segment) = it.next() {
            result = .Some(segment)
        }
        result
    }

    /// Collects all segments into an array of owned strings. (A segment
    /// borrows the source string, so an array of slices can't exist.)
    public func collect() -> Array[String] {
        var result = Array[String]();
        for segment in self {
            result.append(segment.toOwned())
        }
        result
    }
}
