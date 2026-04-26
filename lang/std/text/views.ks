// String views for different representations

module std.text

import std.core.(Bool, Equatable, Comparable, Ordering)
import std.num.(Int64, UInt8, UInt32)
import std.result.(Optional)
import std.iter.(Iterator, Iterable)
import std.text.(Char, Grapheme, Byte, decodeUtf8, String)
import std.text.unicode.(GraphemeBreakProperty, graphemeBreakProperty, shouldBreakBetween)
import std.collections.(Array)

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
            self.index = self.index + Int64(intLiteral: 1);
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
/// s.bytes.count();             // 2
/// s.bytes.byteAt(index: 0);    // Some(104)
/// s.bytes.byteAt(index: 5);    // None (out of bounds)
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
    type Iter = BytesIterator

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

    /// Returns the number of bytes in the view.
    ///
    /// Constant time. Note this is **byte** count, not character count
    /// — see `CharsView.count()` for the latter.
    public func count() -> Int64 { self.length }

    /// Returns true if the view spans zero bytes.
    public func isEmpty() -> Bool { self.length == Int64(intLiteral: 0) }

    /// Returns the raw pointer to the underlying byte buffer.
    ///
    /// Intended for FFI bridges; the pointer is only valid as long as
    /// the source string remains live and unmutated.
    public func asRaw() -> lang.ptr[lang.i8] { self.ptr }

    /// Returns the byte at `index`, or `None` if out of range.
    ///
    /// O(1). For unchecked access in tight loops, use
    /// `byteAtUnchecked`.
    ///
    /// # Examples
    ///
    /// ```
    /// "abc".bytes.byteAt(index: 0);   // Some(97)
    /// "abc".bytes.byteAt(index: 9);   // None
    /// ```
    public func byteAt(index: Int64) -> UInt8? {
        if index >= Int64(intLiteral: 0) and index < self.length {
            let rawOffset: lang.i64 = index.raw;
            let bytePtr: lang.ptr[lang.i8] = lang.ptr_offset[lang.i8](self.ptr, rawOffset);
            let signedByte: lang.i8 = lang.ptr_read(bytePtr);
            .Some(UInt8(raw: signedByte))
        } else {
            .None
        }
    }

    /// Returns the byte at `index` without bounds checking.
    ///
    /// # Safety
    ///
    /// Caller must guarantee `0 <= index < count()`. Out-of-range
    /// access reads arbitrary memory.
    public func byteAtUnchecked(index: Int64) -> UInt8 {
        let rawOffset: lang.i64 = index.raw;
        let bytePtr: lang.ptr[lang.i8] = lang.ptr_offset[lang.i8](self.ptr, rawOffset);
        let signedByte: lang.i8 = lang.ptr_read(bytePtr);
        UInt8(raw: signedByte)
    }

    /// Returns the substring spanning byte indices `[start, end)`.
    ///
    /// Panics if the range is out of bounds, inverted, or splits a
    /// multi-byte UTF-8 character. For a non-panicking variant, use
    /// the `(checked:to:)` overload.
    ///
    /// # Errors
    ///
    /// Panics with `"BytesView.substring: invalid range or UTF-8 boundary"`
    /// if `start < 0`, `end > count()`, `start > end`, or either bound
    /// falls inside a multi-byte sequence.
    ///
    /// # Examples
    ///
    /// ```
    /// "hello".bytes.substring(from: 1, to: 4);  // "ell"
    /// ```
    public func substring(from start: Int64, to end: Int64) -> String {
        match self.substring(checked: start, to: end) {
            .Some(s) => s,
            .None => lang.panic("BytesView.substring: invalid range or UTF-8 boundary")
        }
    }

    /// Returns the substring spanning byte indices `[start, end)`, or `None` if invalid.
    ///
    /// Validates that both bounds lie at UTF-8 character boundaries
    /// (not on a continuation byte) before constructing the result.
    /// Empty ranges (`start == end`) are accepted and yield the empty
    /// string.
    ///
    /// # Examples
    ///
    /// ```
    /// "hello".bytes.substring(checked: 1, to: 4);   // Some("ell")
    /// "hello".bytes.substring(checked: 4, to: 1);   // None (inverted)
    /// "hello".bytes.substring(checked: 0, to: 99);  // None (out of bounds)
    /// ```
    public func substring(checked start: Int64, to end: Int64) -> String? {
        // Bounds check
        if start < Int64(intLiteral: 0) or end > self.length or start > end {
            return .None
        }
        // Check that start is at a valid UTF-8 boundary (not a continuation byte)
        if start > Int64(intLiteral: 0) and start < self.length {
            let startByte = self.byteAtUnchecked(start);
            let startVal: lang.i32 = lang.cast_i8_i32(startByte.raw);
            // Continuation bytes have pattern 10xxxxxx (0x80-0xBF)
            if Bool(boolLiteral: lang.i32_eq(lang.i32_and(startVal, 0xC0), 0x80)) {
                return .None
            }
        }
        // Check that end is at a valid UTF-8 boundary
        if end > Int64(intLiteral: 0) and end < self.length {
            let endByte = self.byteAtUnchecked(end);
            let endVal: lang.i32 = lang.cast_i8_i32(endByte.raw);
            if Bool(boolLiteral: lang.i32_eq(lang.i32_and(endVal, 0xC0), 0x80)) {
                return .None
            }
        }
        // Create substring
        let count = end - start;
        if count == Int64(intLiteral: 0) {
            return .Some(String())
        }
        var result = String(capacity: count);
        for i in start..<end {
            result.appendByte(self.byteAtUnchecked(i))
        }
        .Some(result)
    }

    /// Returns a `BytesIterator` positioned at byte 0.
    ///
    /// Required by `Iterable`. Each call produces a fresh iterator —
    /// the view is reusable.
    public func iter() -> BytesIterator {
        BytesIterator(ptr: self.ptr, length: self.length, index: Int64(intLiteral: 0))
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
            self.byteIndex = self.byteIndex + Int64(intLiteral: 1);
            let replacementValue = UInt32(raw: 0xFFFD);
            .Some(Char(replacementValue))
        }
    }
}

/// A view over the Unicode code points in a `String`.
///
/// Returned by `String.chars`. Iteration is O(1) per code point but
/// `count()` is O(n) because UTF-8 is variable-width. Likewise,
/// `substring(from:to:)` is O(n) — to index in O(1), use
/// `BytesView` and convert byte offsets back yourself.
///
/// # Examples
///
/// ```
/// let s = "héllo";
/// s.chars.count();                       // 5 (code points)
/// s.bytes.count();                       // 6 (bytes — 'é' is 2 bytes)
/// s.chars.substring(from: 1, to: 4);     // "éll"
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
    type Iter = CharsIterator

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
        CharsIterator(ptr: self.ptr, length: self.length, byteIndex: Int64(intLiteral: 0))
    }

    /// Returns the number of code points (O(n)).
    ///
    /// Walks the buffer counting UTF-8 leading bytes (those whose top
    /// two bits are not `10`). For ASCII strings this is exactly
    /// `byteCount`. Cache the result if you need it more than once.
    public func count() -> Int64 {
        var n: Int64 = Int64(intLiteral: 0);
        for i in Int64(intLiteral: 0)..<self.length {
            // Count leading bytes only (not continuation bytes 10xxxxxx)
            let rawOffset: lang.i64 = i.raw;
            let bytePtr: lang.ptr[lang.i8] = lang.ptr_offset[lang.i8](self.ptr, rawOffset);
            let signedByte: lang.i8 = lang.ptr_read(bytePtr);
            let byteVal: lang.i32 = lang.cast_i8_i32(signedByte);
            if lang.i32_ne(lang.i32_and(byteVal, 0xC0), 0x80) {
                n = n + Int64(intLiteral: 1)
            }
        }
        n
    }

    /// Returns the substring spanning code-point indices `[start, end)`. O(n).
    ///
    /// Panics if the range is out of bounds. For a non-panicking
    /// variant, use `(checked:to:)`.
    ///
    /// # Errors
    ///
    /// Panics with `"CharsView.substring: index out of bounds"` if
    /// `start < 0`, `start > end`, or `end` exceeds the code-point
    /// count.
    ///
    /// # Examples
    ///
    /// ```
    /// "héllo".chars.substring(from: 1, to: 3);  // "él"
    /// ```
    public func substring(from start: Int64, to end: Int64) -> String {
        match self.substring(checked: start, to: end) {
            .Some(s) => s,
            .None => lang.panic("CharsView.substring: index out of bounds")
        }
    }

    /// Returns the substring spanning code-point indices `[start, end)`, or `None` if out of range. O(n).
    ///
    /// Walks the buffer twice in the worst case to find the byte
    /// offsets of the two endpoints, then copies the bytes between
    /// them. Empty ranges and ranges that hit the very end of the
    /// string are accepted.
    ///
    /// # Examples
    ///
    /// ```
    /// "héllo".chars.substring(checked: 1, to: 3);  // Some("él")
    /// "héllo".chars.substring(checked: 0, to: 99); // None
    /// ```
    public func substring(checked start: Int64, to end: Int64) -> String? {
        if start < Int64(intLiteral: 0) or start > end {
            return .None
        }

        // Find byte offsets for start and end character indices
        var charIndex: Int64 = Int64(intLiteral: 0);
        var byteIndex: Int64 = Int64(intLiteral: 0);
        var startByte: Int64 = Int64(intLiteral: 0);
        var endByte: Int64 = Int64(intLiteral: 0);
        var foundStart: Bool = false;
        var foundEnd: Bool = false;

        // Handle empty range at start
        if start == Int64(intLiteral: 0) {
            startByte = Int64(intLiteral: 0);
            foundStart = true
        }
        if end == Int64(intLiteral: 0) {
            endByte = Int64(intLiteral: 0);
            foundEnd = true
        }

        while byteIndex < self.length and foundEnd == false {
            let result = decodeUtf8(self.ptr, self.length, at: byteIndex);
            if let .Some(decoded) = result {
                if charIndex == start and foundStart == false {
                    startByte = byteIndex;
                    foundStart = true
                }
                charIndex = charIndex + Int64(intLiteral: 1);
                byteIndex = byteIndex + decoded.bytesConsumed;
                if charIndex == end {
                    endByte = byteIndex;
                    foundEnd = true
                }
            } else {
                byteIndex = byteIndex + Int64(intLiteral: 1)
            }
        }

        // Handle end at string end
        if foundStart and end == charIndex and foundEnd == false {
            endByte = byteIndex;
            foundEnd = true
        }

        // Handle empty range at non-zero offset (start == end > 0)
        if foundEnd and foundStart == false and start == end {
            foundStart = true;
            startByte = endByte
        }

        if foundStart == false or foundEnd == false {
            return .None
        }

        // Create substring from byte range
        let count = endByte - startByte;
        if count == Int64(intLiteral: 0) {
            return .Some(String())
        }
        var result = String(capacity: count);
        for i in startByte..<endByte {
            let rawOffset: lang.i64 = i.raw;
            let bytePtr: lang.ptr[lang.i8] = lang.ptr_offset[lang.i8](self.ptr, rawOffset);
            let signedByte: lang.i8 = lang.ptr_read(bytePtr);
            result.appendByte(UInt8(raw: signedByte))
        }
        .Some(result)
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
    private var prevProp: GraphemeBreakProperty
    private var prevPrevWasRI: Bool
    private var started: Bool

    /// @name From Chars
    /// Wraps a `CharsIterator` to produce graphemes via UAX #29 segmentation.
    ///
    /// Prefer `someString.graphemes.iter()` over calling this directly.
    public init(charsIter: CharsIterator) {
        self.charsIter = charsIter;
        self.pendingChar = .None;
        self.prevProp = GraphemeBreakProperty.Other;
        self.prevPrevWasRI = false;
        self.started = false;
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
        if chars.count == Int64(intLiteral: 1) {
            .Some(Grapheme(char: chars(unchecked: Int64(intLiteral: 0))))
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
/// flag.chars.count();      // 2 (regional indicators)
/// flag.graphemes.count();  // 1 (one flag)
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
    type Iter = GraphemesIterator

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
        GraphemesIterator(CharsIterator(ptr: self.ptr, length: self.length, byteIndex: Int64(intLiteral: 0)))
    }

    /// Returns the number of grapheme clusters (O(n)).
    ///
    /// Walks the entire string through the UAX #29 segmenter. Cache
    /// the result if you need it more than once.
    public func count() -> Int64 {
        var n: Int64 = Int64(intLiteral: 0);
        for _ in self.iter() {
            n = n + Int64(intLiteral: 1)
        }
        n
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
                self.byteIndex = self.byteIndex + Int64(intLiteral: 1);
                foundNewline = true
            } else if Bool(boolLiteral: lang.i32_eq(unsignedByte, 13)) {  // \r
                lineEnd = self.byteIndex;
                self.byteIndex = self.byteIndex + Int64(intLiteral: 1);
                // Handle \r\n
                if self.byteIndex < self.length {
                    let nextOffset: lang.i64 = self.byteIndex.raw;
                    let nextBytePtr: lang.ptr[lang.i8] = lang.ptr_offset[lang.i8](self.ptr, nextOffset);
                    let nextByte: lang.i8 = lang.ptr_read(nextBytePtr);
                    let nextByteVal: lang.i32 = lang.cast_i8_i32(nextByte);
                    let nextUnsigned: lang.i32 = lang.i32_and(nextByteVal, 0xFF);
                    if Bool(boolLiteral: lang.i32_eq(nextUnsigned, 10)) {
                        self.byteIndex = self.byteIndex + Int64(intLiteral: 1)
                    }
                }
                foundNewline = true
            } else {
                self.byteIndex = self.byteIndex + Int64(intLiteral: 1)
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

    /// Copies bytes `[start, end)` into a fresh `String`.
    private func createSubstring(start: Int64, end: Int64) -> String {
        let count = end - start;
        if count == Int64(intLiteral: 0) {
            return String()
        }
        // Create string from bytes
        var result = String(capacity: count);
        for currentIndex in start..<end {
            let rawOffset: lang.i64 = currentIndex.raw;
            let bytePtr: lang.ptr[lang.i8] = lang.ptr_offset[lang.i8](self.ptr, rawOffset);
            let byte: lang.i8 = lang.ptr_read(bytePtr);
            result.appendByte(UInt8(raw: byte))
        }
        result
    }
}

/// A view over the lines of a `String`, split on `\n`, `\r\n`, or `\r`.
///
/// Returned by `String.lines`. The yielded strings do not include
/// the terminator; a trailing line without a terminator is still
/// emitted. To re-join with a specific separator, collect the
/// iterator output and use `String.append`.
///
/// # Examples
///
/// ```
/// var lines = Array[String]();
/// for line in "a\nb\nc".lines {
///     lines.append(line);
/// }
/// lines.count;  // 3
/// ```
///
/// # Representation
///
/// A `(ptr, length)` pair pointing into the source string.
public struct LinesView: Iterable {
    /// The element type yielded by iteration — always `String`.
    type Item = String
    /// The iterator type returned by `iter()`.
    type Iter = LinesIterator

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
        LinesIterator(ptr: self.ptr, length: self.length, byteIndex: Int64(intLiteral: 0), done: false)
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
    public func equals(other: ByteIndex) -> Bool {
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
    public func equals(other: CharIndex) -> Bool {
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
    public func equals(other: GraphemeIndex) -> Bool {
        self.byteOffset == other.byteOffset
    }
}
