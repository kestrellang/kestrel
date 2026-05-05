// Str protocol - shared read-only API for String and StringSlice

module std.text

import std.core.(Bool, Equatable, Comparable, Ordering, Hashable, Hasher, fatalError)
import std.numeric.(Int64, UInt8)
import std.result.(Optional)
import std.memory.(Pointer, RawPointer)
import std.iter.(Iterable)
import std.ffi.(memmem)
import std.text.(Formattable, FormatOptions, Char, decodeUtf8, String, StringBuilder, StringSlice, CharsIterator, BytesView, CharsView, GraphemesView, LinesView, ByteIndex, CharIndex, GraphemeIndex, LineIndex, SplitView, SplitWhereView, _bytesEqual)
import std.text.unicode as unicode

// ============================================================================
// STR PROTOCOL
// ============================================================================

/// Shared read-only protocol for `String` and `StringSlice`.
///
/// Requires exactly one method from conformers: `asSlice()`. All
/// read-only methods are defined once in `extend Str` and inherited
/// by both types automatically.
public protocol Str: Iterable, Equatable, Comparable, Hashable, Formattable {
    func asSlice() -> StringSlice
}

// ============================================================================
// EXTEND STR — Read-Only Methods
// ============================================================================

extend Str {

    // -- Size ----------------------------------------------------------------

    /// Number of UTF-8 bytes. O(1).
    ///
    /// # Examples
    ///
    /// ```
    /// "hello".byteCount;       // 5
    /// "\u{00E9}".byteCount;    // 2 (é is two UTF-8 bytes)
    /// ```
    public var byteCount: Int64 { self.asSlice().byteCount }

    /// True when the string contains no bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// "".isEmpty;       // true
    /// "hello".isEmpty;  // false
    /// ```
    public var isEmpty: Bool { self.asSlice().isEmpty }

    // -- Views ---------------------------------------------------------------

    /// View over the raw UTF-8 bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// "hi".bytes.count;  // 2
    /// ```
    public var bytes: BytesView { BytesView(slice: self.asSlice()) }

    /// View over Unicode code points.
    ///
    /// # Examples
    ///
    /// ```
    /// "caf\u{00E9}".chars.count;  // 4
    /// ```
    public var chars: CharsView { CharsView(slice: self.asSlice()) }

    /// View over grapheme clusters (user-perceived characters).
    ///
    /// # Examples
    ///
    /// ```
    /// "caf\u{00E9}".graphemes.count;  // 4
    /// ```
    public var graphemes: GraphemesView { GraphemesView(slice: self.asSlice()) }

    /// View over lines, recognising `\n`, `\r\n`, and `\r`.
    ///
    /// # Examples
    ///
    /// ```
    /// "a\nb\nc".lines.count;  // 3
    /// ```
    public var lines: LinesView { LinesView(slice: self.asSlice()) }

    // -- Conversion ----------------------------------------------------------

    /// Copies this string's bytes into a new independent `String`.
    ///
    /// For `String`, this is equivalent to `clone()`. For
    /// `StringSlice`, it copies only the slice's bytes, releasing
    /// the reference to the source buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// let slice = "hello world".asSlice();
    /// let owned = slice.toOwned();  // independent copy
    /// ```
    public func toOwned() -> String {
        self.asSlice().toOwned()
    }

    // -- Iteration -----------------------------------------------------------

    /// Returns a `CharsIterator` over the code points.
    ///
    /// Required by `Iterable`. Each call returns a fresh iterator;
    /// the source is reusable.
    ///
    /// # Examples
    ///
    /// ```
    /// for c in "abc" { ... }  // iterates 'a', 'b', 'c'
    /// ```
    public func iter() -> CharsIterator {
        self.asSlice().iter()
    }

    // -- Protocol conformances -----------------------------------------------

    /// Returns true if both strings have the same byte sequence.
    ///
    /// Pure byte-wise equality — not normalization-aware. For
    /// case-insensitive comparison, see `equalsCaseInsensitive`.
    ///
    /// # Examples
    ///
    /// ```
    /// "abc".isEqual(to: "abc");  // true
    /// "abc".isEqual(to: "ABC");  // false
    /// ```
    public func isEqual(to other: Self) -> Bool {
        self.asSlice().isEqual(to: other.asSlice())
    }

    /// Lexicographic byte-wise comparison.
    ///
    /// Returns `Less` / `Equal` / `Greater` according to the first
    /// differing byte; if one string is a prefix of the other, the
    /// shorter is less.
    ///
    /// # Examples
    ///
    /// ```
    /// "abc".compare("abd");  // Less
    /// "abc".compare("ab");   // Greater
    /// ```
    public func compare(other: Self) -> Ordering {
        self.asSlice().compare(other.asSlice())
    }

    /// Hashes the byte content into `hasher`.
    public func hash[H](mutating into hasher: H) where H: Hasher {
        self.asSlice().hash(into: hasher)
    }

    /// Formats the string using the given options.
    ///
    /// # Examples
    ///
    /// ```
    /// "hi".format(FormatOptions(width: 5));  // "hi   "
    /// ```
    public func format(mutating into writer: StringBuilder, options: FormatOptions = FormatOptions.default()) {
        self.toOwned().format(into: writer, options)
    }

    // -- Searching -------------------------------------------------------------

    /// Returns true if `substring` appears anywhere in this string.
    ///
    /// # Examples
    ///
    /// ```
    /// "hello world".contains(substring: "world");  // true
    /// "hello world".contains(substring: "xyz");    // false
    /// ```
    public func contains(substring: String) -> Bool {
        self.firstIndex(of: substring).isSome()
    }

    /// Returns true if any code point matches `predicate`.
    ///
    /// # Examples
    ///
    /// ```
    /// "abc123".contains(matching: { (c) in c.isAsciiDigit });  // true
    /// ```
    public func contains(matching predicate: (Char) -> Bool) -> Bool {
        self.chars.firstIndex(matching: predicate).isSome()
    }

    /// Returns true if this string starts with `prefix`.
    ///
    /// Empty prefix always returns true. Comparison is byte-wise.
    ///
    /// # Examples
    ///
    /// ```
    /// "hello".starts(with: "hel");  // true
    /// "hello".starts(with: "xyz");  // false
    /// ```
    public func starts(with prefix: String) -> Bool {
        let slice = self.asSlice();
        let ps = prefix.asSlice();
        let prefixLen = ps.byteCount;
        if prefixLen > slice.byteCount { return false }
        if prefixLen == 0 { return true }
        _bytesEqual(a: slice._rawPtr().offset(by: slice.start), b: ps._rawPtr().offset(by: ps.start), n: prefixLen)
    }

    /// Returns true if this string ends with `suffix`.
    ///
    /// Empty suffix always returns true. Comparison is byte-wise.
    ///
    /// # Examples
    ///
    /// ```
    /// "hello".ends(with: "llo");  // true
    /// "hello".ends(with: "xyz");  // false
    /// ```
    public func ends(with suffix: String) -> Bool {
        let slice = self.asSlice();
        let ss = suffix.asSlice();
        let suffixLen = ss.byteCount;
        if suffixLen > slice.byteCount { return false }
        if suffixLen == 0 { return true }
        _bytesEqual(a: slice._rawPtr().offset(by: slice.end - suffixLen), b: ss._rawPtr().offset(by: ss.start), n: suffixLen)
    }

    /// Returns the byte index of the first occurrence of `substring`,
    /// or `None` if not found.
    ///
    /// The empty substring matches at the start. Uses `memmem` for
    /// efficient byte-level search.
    ///
    /// # Examples
    ///
    /// ```
    /// "hello world".firstIndex(of: "world");  // Some(ByteIndex(6))
    /// "hello world".firstIndex(of: "xyz");    // None
    /// ```
    public func firstIndex(of substring: String) -> ByteIndex? {
        let slice = self.asSlice();
        let sub = substring.asSlice();
        let subLen = sub.byteCount;
        let myLen = slice.byteCount;
        if subLen == 0 {
            return .Some(ByteIndex(slice.start))
        }
        if subLen > myLen { return .None }
        let base = slice._rawPtr().offset(by: slice.start).asRaw();
        let needle = sub._rawPtr().offset(by: sub.start).asRaw();
        let result = memmem(base, myLen, needle, subLen);
        if result.isNull {
            .None
        } else {
            let diff: lang.i64 = lang.i64_sub(result.address.raw, base.address.raw);
            .Some(ByteIndex(slice.start + Int64(intLiteral: diff)))
        }
    }

    /// Returns the byte index of the last occurrence of `substring`,
    /// or `None` if not found.
    ///
    /// Scans from the left using repeated `memmem` calls, keeping
    /// the last match position.
    ///
    /// # Examples
    ///
    /// ```
    /// "abcabc".lastIndex(of: "abc");  // Some(ByteIndex(3))
    /// "abcabc".lastIndex(of: "xyz");  // None
    /// ```
    public func lastIndex(of substring: String) -> ByteIndex? {
        let slice = self.asSlice();
        let sub = substring.asSlice();
        let subLen = sub.byteCount;
        let myLen = slice.byteCount;
        if subLen == 0 {
            return .Some(ByteIndex(slice.end))
        }
        if subLen > myLen { return .None }
        let myPtr = slice._rawPtr();
        let needlePtr = sub._rawPtr().offset(by: sub.start).asRaw();
        var lastFound: Int64 = -1;
        var i: Int64 = 0;
        while myLen - i >= subLen {
            let base = myPtr.offset(by: slice.start + i).asRaw();
            let remaining = myLen - i;
            let result = memmem(base, remaining, needlePtr, subLen);
            if result.isNull {
                break
            }
            let diff: lang.i64 = lang.i64_sub(result.address.raw, base.address.raw);
            let matchIndex = i + Int64(intLiteral: diff);
            lastFound = matchIndex;
            i = matchIndex + 1
        }
        if lastFound < 0 {
            .None
        } else {
            .Some(ByteIndex(slice.start + lastFound))
        }
    }

    // -- Splitting -------------------------------------------------------------

    /// Returns a lazy view that splits on `separator`, yielding
    /// zero-copy `StringSlice` segments.
    ///
    /// The empty separator is special-cased to split per code
    /// point. Adjacent separators produce empty segments.
    ///
    /// # Examples
    ///
    /// ```
    /// "a,b,c".split(",").collect();   // ["a", "b", "c"]
    /// "a,,b".split(",").count;        // 3 (empty segment preserved)
    /// ```
    public func split(separator: String) -> SplitView {
        SplitView(slice: self.asSlice(), separator: separator)
    }

    /// Returns a lazy view that splits at every code point matching
    /// `predicate`, yielding zero-copy `StringSlice` segments.
    ///
    /// The matching characters are not included in any segment.
    ///
    /// # Examples
    ///
    /// ```
    /// "hello world".split(matching: { (c) in c.isWhitespace }).count;  // 2
    /// ```
    public func split(matching predicate: (Char) -> Bool) -> SplitWhereView {
        SplitWhereView(slice: self.asSlice(), matching: predicate)
    }

    // -- Trimming (non-mutating) -----------------------------------------------

    /// Returns a zero-copy slice with leading and trailing ASCII
    /// whitespace removed.
    ///
    /// Whitespace characters: space (`' '`), tab (`'\t'`), newline
    /// (`'\n'`), carriage return (`'\r'`), and form feed (`'\x0C'`).
    /// The returned `StringSlice` shares the source buffer — no
    /// allocation occurs.
    ///
    /// # Examples
    ///
    /// ```
    /// "  hello  ".trimmed().toOwned();   // "hello"
    /// "\t\n".trimmed().isEmpty;          // true
    /// ```
    public func trimmed() -> StringSlice {
        let slice = self.asSlice();
        let myLen = slice.byteCount;
        let basePtr = slice._rawPtr().offset(by: slice.start);
        var realStart: Int64 = 0;
        var startDone: Bool = false;
        while realStart < myLen and startDone == false {
            let byte = basePtr.offset(by: realStart).read();
            let v: lang.i32 = lang.cast_i8_i32(byte.raw);
            let isWs = lang.i1_or(lang.i1_or(lang.i1_or(lang.i1_or(lang.i32_eq(v, 32), lang.i32_eq(v, 9)), lang.i32_eq(v, 10)), lang.i32_eq(v, 13)), lang.i32_eq(v, 12));
            if Bool(boolLiteral: isWs) {
                realStart = realStart + 1
            } else {
                startDone = true
            }
        }
        var endPos: Int64 = myLen;
        var endDone: Bool = false;
        while endPos > realStart and endDone == false {
            let idx = endPos - 1;
            let byte = basePtr.offset(by: idx).read();
            let v: lang.i32 = lang.cast_i8_i32(byte.raw);
            let isWs = lang.i1_or(lang.i1_or(lang.i1_or(lang.i1_or(lang.i32_eq(v, 32), lang.i32_eq(v, 9)), lang.i32_eq(v, 10)), lang.i32_eq(v, 13)), lang.i32_eq(v, 12));
            if Bool(boolLiteral: isWs) {
                endPos = endPos - 1
            } else {
                endDone = true
            }
        }
        slice.subslice(from: slice.start + realStart, to: slice.start + endPos)
    }

    /// Returns a zero-copy slice with leading whitespace removed.
    ///
    /// See `trimmed()` for the whitespace set. Trailing whitespace
    /// is preserved.
    ///
    /// # Examples
    ///
    /// ```
    /// "  hello  ".trimmedStart().toOwned();  // "hello  "
    /// ```
    public func trimmedStart() -> StringSlice {
        let slice = self.asSlice();
        let myLen = slice.byteCount;
        let basePtr = slice._rawPtr().offset(by: slice.start);
        var realStart: Int64 = 0;
        var done: Bool = false;
        while realStart < myLen and done == false {
            let byte = basePtr.offset(by: realStart).read();
            let v: lang.i32 = lang.cast_i8_i32(byte.raw);
            let isWs = lang.i1_or(lang.i1_or(lang.i1_or(lang.i1_or(lang.i32_eq(v, 32), lang.i32_eq(v, 9)), lang.i32_eq(v, 10)), lang.i32_eq(v, 13)), lang.i32_eq(v, 12));
            if Bool(boolLiteral: isWs) {
                realStart = realStart + 1
            } else {
                done = true
            }
        }
        slice.subslice(from: slice.start + realStart, to: slice.end)
    }

    /// Returns a zero-copy slice with trailing whitespace removed.
    ///
    /// See `trimmed()` for the whitespace set. Leading whitespace
    /// is preserved.
    ///
    /// # Examples
    ///
    /// ```
    /// "  hello  ".trimmedEnd().toOwned();  // "  hello"
    /// ```
    public func trimmedEnd() -> StringSlice {
        let slice = self.asSlice();
        let myLen = slice.byteCount;
        let basePtr = slice._rawPtr().offset(by: slice.start);
        var endPos: Int64 = myLen;
        var done: Bool = false;
        while endPos > 0 and done == false {
            let idx = endPos - 1;
            let byte = basePtr.offset(by: idx).read();
            let v: lang.i32 = lang.cast_i8_i32(byte.raw);
            let isWs = lang.i1_or(lang.i1_or(lang.i1_or(lang.i1_or(lang.i32_eq(v, 32), lang.i32_eq(v, 9)), lang.i32_eq(v, 10)), lang.i32_eq(v, 13)), lang.i32_eq(v, 12));
            if Bool(boolLiteral: isWs) {
                endPos = endPos - 1
            } else {
                done = true
            }
        }
        slice.subslice(from: slice.start, to: slice.start + endPos)
    }

    /// Returns a zero-copy slice with leading and trailing code points
    /// matching `predicate` removed.
    ///
    /// Decodes the source one `Char` at a time. Leading characters
    /// that satisfy the predicate are skipped; the trailing boundary
    /// is the last character that does *not* match.
    ///
    /// # Examples
    ///
    /// ```
    /// "00042".trimmed(matching: { (c) in c.isEqual(to: '0') }).toOwned();  // "42"
    /// ```
    public func trimmed(matching predicate: (Char) -> Bool) -> StringSlice {
        let slice = self.asSlice();
        let myLen = slice.byteCount;
        let basePtr = slice._rawPtr().offset(by: slice.start);
        let rawPtr: lang.ptr[lang.i8] = lang.cast_ptr[_, lang.i8](basePtr.asRaw().raw);
        var realStart: Int64 = 0;
        var startDone: Bool = false;
        while realStart < myLen and startDone == false {
            let result = decodeUtf8(rawPtr, myLen, at: realStart);
            if let .Some(decoded) = result {
                if predicate(decoded.char) {
                    realStart = realStart + decoded.bytesConsumed
                } else {
                    startDone = true
                }
            } else {
                startDone = true
            }
        }
        var lastNonMatch: Int64 = realStart;
        var i: Int64 = realStart;
        while i < myLen {
            let result = decodeUtf8(rawPtr, myLen, at: i);
            if let .Some(decoded) = result {
                if predicate(decoded.char) == false {
                    lastNonMatch = i + decoded.bytesConsumed
                }
                i = i + decoded.bytesConsumed
            } else {
                i = i + 1
            }
        }
        slice.subslice(from: slice.start + realStart, to: slice.start + lastNonMatch)
    }

    /// Returns a zero-copy slice with leading code points matching
    /// `predicate` removed. Trailing matches are preserved.
    ///
    /// # Examples
    ///
    /// ```
    /// "000abc".trimmedStart(matching: { (c) in c.isEqual(to: '0') }).toOwned();  // "abc"
    /// ```
    public func trimmedStart(matching predicate: (Char) -> Bool) -> StringSlice {
        let slice = self.asSlice();
        let myLen = slice.byteCount;
        let basePtr = slice._rawPtr().offset(by: slice.start);
        let rawPtr: lang.ptr[lang.i8] = lang.cast_ptr[_, lang.i8](basePtr.asRaw().raw);
        var realStart: Int64 = 0;
        var done: Bool = false;
        while realStart < myLen and done == false {
            let result = decodeUtf8(rawPtr, myLen, at: realStart);
            if let .Some(decoded) = result {
                if predicate(decoded.char) {
                    realStart = realStart + decoded.bytesConsumed
                } else {
                    done = true
                }
            } else {
                done = true
            }
        }
        slice.subslice(from: slice.start + realStart, to: slice.end)
    }

    /// Returns a zero-copy slice with trailing code points matching
    /// `predicate` removed. Leading matches are preserved.
    ///
    /// # Examples
    ///
    /// ```
    /// "abc000".trimmedEnd(matching: { (c) in c.isEqual(to: '0') }).toOwned();  // "abc"
    /// ```
    public func trimmedEnd(matching predicate: (Char) -> Bool) -> StringSlice {
        let slice = self.asSlice();
        let myLen = slice.byteCount;
        let basePtr = slice._rawPtr().offset(by: slice.start);
        let rawPtr: lang.ptr[lang.i8] = lang.cast_ptr[_, lang.i8](basePtr.asRaw().raw);
        var lastNonMatch: Int64 = 0;
        var i: Int64 = 0;
        while i < myLen {
            let result = decodeUtf8(rawPtr, myLen, at: i);
            if let .Some(decoded) = result {
                if predicate(decoded.char) == false {
                    lastNonMatch = i + decoded.bytesConsumed
                }
                i = i + decoded.bytesConsumed
            } else {
                i = i + 1
            }
        }
        slice.subslice(from: slice.start, to: slice.start + lastNonMatch)
    }

    // -- Case conversion -------------------------------------------------------

    /// Returns the lowercase form using full Unicode case mapping.
    ///
    /// Locale-independent. Handles multi-character expansions
    /// (e.g. Turkish dotted I). All-ASCII strings with no uppercase
    /// letters short-circuit to `toOwned()` (no per-char decode).
    ///
    /// # Examples
    ///
    /// ```
    /// "Hello".lowercased();      // "hello"
    /// "\u{0130}".lowercased();   // "i\u{0307}"
    /// ```
    public func lowercased() -> String {
        let slice = self.asSlice();
        let myLen = slice.byteCount;
        let basePtr = slice._rawPtr().offset(by: slice.start);
        var hasUpperAscii = false;
        var i: Int64 = 0;
        while i < myLen {
            let byte = basePtr.offset(by: i).read();
            if byte > 127 {
                var result = String();
                for c in self.chars.iter() {
                    if unicode.hasLowercaseExpansion(c) {
                        result.append(unicode.lowercaseExpansion(c))
                    } else {
                        result.appendChar(unicode.toLowercase(c))
                    }
                }
                return result
            }
            if byte >= 65 and byte <= 90 {
                hasUpperAscii = true
            }
            i = i + 1
        }
        if hasUpperAscii == false {
            return self.toOwned()
        }
        var result = String(capacity: myLen);
        i = 0;
        while i < myLen {
            let byte = basePtr.offset(by: i).read();
            let v: lang.i32 = lang.cast_i8_i32(byte.raw);
            let isUpper = lang.i1_and(lang.i32_signed_ge(v, 65), lang.i32_signed_le(v, 90));
            if Bool(boolLiteral: isUpper) {
                result.appendByte(UInt8(raw: lang.cast_i32_i8(lang.i32_add(v, 32))))
            } else {
                result.appendByte(byte)
            }
            i = i + 1
        }
        result
    }

    /// Returns the uppercase form using full Unicode case mapping.
    ///
    /// Locale-independent. Handles multi-character expansions
    /// (e.g. `ß` → `SS`). All-ASCII strings with no lowercase
    /// letters short-circuit to `toOwned()`.
    ///
    /// # Examples
    ///
    /// ```
    /// "hello".uppercased();             // "HELLO"
    /// "stra\u{00DF}e".uppercased();     // "STRASSE"
    /// ```
    public func uppercased() -> String {
        let slice = self.asSlice();
        let myLen = slice.byteCount;
        let basePtr = slice._rawPtr().offset(by: slice.start);
        var hasLowerAscii = false;
        var i: Int64 = 0;
        while i < myLen {
            let byte = basePtr.offset(by: i).read();
            if byte > 127 {
                var result = String();
                for c in self.chars.iter() {
                    if unicode.hasUppercaseExpansion(c) {
                        result.append(unicode.uppercaseExpansion(c))
                    } else {
                        result.appendChar(unicode.toUppercase(c))
                    }
                }
                return result
            }
            if byte >= 97 and byte <= 122 {
                hasLowerAscii = true
            }
            i = i + 1
        }
        if hasLowerAscii == false {
            return self.toOwned()
        }
        var result = String(capacity: myLen);
        i = 0;
        while i < myLen {
            let byte = basePtr.offset(by: i).read();
            let v: lang.i32 = lang.cast_i8_i32(byte.raw);
            let isLower = lang.i1_and(lang.i32_signed_ge(v, 97), lang.i32_signed_le(v, 122));
            if Bool(boolLiteral: isLower) {
                result.appendByte(UInt8(raw: lang.cast_i32_i8(lang.i32_sub(v, 32))))
            } else {
                result.appendByte(byte)
            }
            i = i + 1
        }
        result
    }

    /// Returns the titlecase form using full Unicode case mapping.
    ///
    /// Word boundaries are detected by `Char.isWhitespace`; the
    /// first non-space character of each run is titlecased and the
    /// rest lowercased.
    ///
    /// # Examples
    ///
    /// ```
    /// "hello world".titlecased();  // "Hello World"
    /// "FOO BAR".titlecased();      // "Foo Bar"
    /// ```
    public func titlecased() -> String {
        var result = String();
        var atWordStart = true;
        for c in self.chars.iter() {
            if c.isWhitespace {
                result.appendChar(c);
                atWordStart = true
            } else if atWordStart {
                if unicode.hasTitlecaseExpansion(c) {
                    result.append(unicode.titlecaseExpansion(c))
                } else {
                    result.appendChar(unicode.toTitlecase(c))
                }
                atWordStart = false
            } else {
                if unicode.hasLowercaseExpansion(c) {
                    result.append(unicode.lowercaseExpansion(c))
                } else {
                    result.appendChar(unicode.toLowercase(c))
                }
            }
        }
        result
    }

    /// Returns a copy with only ASCII letters lowercased; non-ASCII
    /// bytes pass through unchanged.
    ///
    /// Cheap byte-level scan with no Unicode tables. For full
    /// Unicode case mapping, use `lowercased()`.
    ///
    /// # Examples
    ///
    /// ```
    /// "H\u{00E9}LLO".lowercasedAscii();  // "h\u{00E9}llo"
    /// ```
    public func lowercasedAscii() -> String {
        let slice = self.asSlice();
        let myLen = slice.byteCount;
        let basePtr = slice._rawPtr().offset(by: slice.start);
        var result = String(capacity: myLen);
        var i: Int64 = 0;
        while i < myLen {
            let byte = basePtr.offset(by: i).read();
            let v: lang.i32 = lang.cast_i8_i32(byte.raw);
            let isUpper = lang.i1_and(lang.i32_signed_ge(v, 65), lang.i32_signed_le(v, 90));
            if Bool(boolLiteral: isUpper) {
                result.appendByte(UInt8(raw: lang.cast_i32_i8(lang.i32_add(v, 32))))
            } else {
                result.appendByte(byte)
            }
            i = i + 1
        }
        result
    }

    /// Returns a copy with only ASCII letters uppercased; non-ASCII
    /// bytes pass through unchanged.
    ///
    /// Cheap byte-level scan with no Unicode tables. For full
    /// Unicode case mapping, use `uppercased()`.
    ///
    /// # Examples
    ///
    /// ```
    /// "h\u{00E9}llo".uppercasedAscii();  // "H\u{00E9}LLO"
    /// ```
    public func uppercasedAscii() -> String {
        let slice = self.asSlice();
        let myLen = slice.byteCount;
        let basePtr = slice._rawPtr().offset(by: slice.start);
        var result = String(capacity: myLen);
        var i: Int64 = 0;
        while i < myLen {
            let byte = basePtr.offset(by: i).read();
            let v: lang.i32 = lang.cast_i8_i32(byte.raw);
            let isLower = lang.i1_and(lang.i32_signed_ge(v, 97), lang.i32_signed_le(v, 122));
            if Bool(boolLiteral: isLower) {
                result.appendByte(UInt8(raw: lang.cast_i32_i8(lang.i32_sub(v, 32))))
            } else {
                result.appendByte(byte)
            }
            i = i + 1
        }
        result
    }

    // -- Case-insensitive comparison -------------------------------------------

    /// Compares two strings for equality after Unicode case folding.
    ///
    /// Folds each string to its case-folded form and compares the
    /// results byte-wise. Not normalization-aware — `é` (`U+00E9`)
    /// and `e\u{0301}` are still considered different.
    ///
    /// # Examples
    ///
    /// ```
    /// "Hello".equalsCaseInsensitive("HELLO");  // true
    /// "Hello".equalsCaseInsensitive("World");  // false
    /// ```
    public func equalsCaseInsensitive(other: String) -> Bool {
        self.caseFolded().isEqual(to: other.caseFolded())
    }

    /// Returns a new string with Unicode case folding applied to
    /// each code point.
    ///
    /// Case folding maps characters to a canonical form suitable
    /// for case-insensitive comparison. Currently single-char folds
    /// only (e.g. `A` → `a`); multi-char expansions like `ß` → `ss`
    /// are not yet supported.
    ///
    /// # Examples
    ///
    /// ```
    /// "Hello".caseFolded();  // "hello"
    /// ```
    public func caseFolded() -> String {
        var result = String(capacity: self.byteCount);
        for c in self.chars {
            result.appendChar(unicode.caseFold(c))
        }
        result
    }

    // -- Replacement -----------------------------------------------------------

    /// Returns a copy with every occurrence of `pattern` replaced
    /// by `replacement`.
    ///
    /// Empty `pattern` is a no-op (returns a copy). Searches
    /// greedily from the left and skips past each replacement so
    /// substituted text is not re-matched.
    ///
    /// # Examples
    ///
    /// ```
    /// "hello world".replaced("o", with: "0");    // "hell0 w0rld"
    /// "abcabc".replaced("ab", with: "ABCD");     // "ABCDcABCDc"
    /// ```
    public func replaced(pattern: String, with replacement: String) -> String {
        let patternSlice = pattern.asSlice();
        let patternLen = patternSlice.byteCount;
        if patternLen == 0 {
            return self.toOwned()
        }
        let slice = self.asSlice();
        let myLen = slice.byteCount;
        if patternLen > myLen {
            return self.toOwned()
        }
        let myPtr = slice._rawPtr().offset(by: slice.start);
        let patternPtr = patternSlice._rawPtr().offset(by: patternSlice.start);

        var matchCount: Int64 = 0;
        var i: Int64 = 0;
        while myLen - i >= patternLen {
            let base = myPtr.offset(by: i).asRaw();
            let r = memmem(base, myLen - i, patternPtr.asRaw(), patternLen);
            if r.isNull {
                break
            }
            let diff: lang.i64 = lang.i64_sub(r.address.raw, base.address.raw);
            i = i + Int64(intLiteral: diff) + patternLen;
            matchCount = matchCount + 1
        }
        if matchCount == 0 {
            return self.toOwned()
        }

        let repSlice = replacement.asSlice();
        let repLen = repSlice.byteCount;
        let repPtr = repSlice._rawPtr().offset(by: repSlice.start);
        let resultLen = myLen - matchCount * patternLen + matchCount * repLen;
        var result = String(capacity: resultLen);
        var runStart: Int64 = 0;
        i = 0;
        while myLen - i >= patternLen {
            let base = myPtr.offset(by: i).asRaw();
            let r = memmem(base, myLen - i, patternPtr.asRaw(), patternLen);
            if r.isNull {
                break
            }
            let diff: lang.i64 = lang.i64_sub(r.address.raw, base.address.raw);
            let matchIndex = i + Int64(intLiteral: diff);
            result._appendBytes(myPtr.offset(by: runStart), matchIndex - runStart);
            result._appendBytes(repPtr, repLen);
            i = matchIndex + patternLen;
            runStart = i
        }
        result._appendBytes(myPtr.offset(by: runStart), myLen - runStart);
        result
    }

    // -- Repeating & padding ---------------------------------------------------

    /// Returns this string concatenated with itself `count` times.
    ///
    /// Non-positive `count` returns the empty string. Pre-allocates
    /// the result buffer for the exact final length.
    ///
    /// # Examples
    ///
    /// ```
    /// "ab".repeated(3);  // "ababab"
    /// "ab".repeated(0);  // ""
    /// ```
    public func repeated(count: Int64) -> String {
        if count <= 0 {
            return String()
        }
        let slice = self.asSlice();
        let myLen = slice.byteCount;
        if myLen == 0 {
            return String()
        }
        let basePtr = slice._rawPtr().offset(by: slice.start);
        let totalLen = myLen * count;
        var result = String(capacity: totalLen);
        for i in 0..<count {
            result._appendBytes(basePtr, myLen)
        }
        result
    }

    /// Returns the string padded at the start with `char` so the
    /// total *code-point* count is at least `length`.
    ///
    /// If the string is already at least `length` code points long,
    /// returns a copy unchanged.
    ///
    /// # Examples
    ///
    /// ```
    /// "42".pad(leading: 5, with: '0');  // "00042"
    /// ```
    public func pad(leading length: Int64, with char: Char) -> String {
        let currentLen = self.chars.count;
        if currentLen >= length {
            return self.toOwned()
        }
        let paddingCount = length - currentLen;
        var result = String(capacity: self.byteCount + paddingCount * char.utf8Length());
        for i in 0..<paddingCount {
            result.appendChar(char)
        }
        let slice = self.asSlice();
        result._appendBytes(slice._rawPtr().offset(by: slice.start), slice.byteCount);
        result
    }

    /// Returns the string padded at the end with `char` so the
    /// total *code-point* count is at least `length`.
    ///
    /// If the string is already at least `length` code points long,
    /// returns a copy unchanged.
    ///
    /// # Examples
    ///
    /// ```
    /// "42".pad(trailing: 5, with: '.');  // "42..."
    /// ```
    public func pad(trailing length: Int64, with char: Char) -> String {
        let currentLen = self.chars.count;
        if currentLen >= length {
            return self.toOwned()
        }
        let paddingCount = length - currentLen;
        var result = String(capacity: self.byteCount + paddingCount * char.utf8Length());
        let slice = self.asSlice();
        result._appendBytes(slice._rawPtr().offset(by: slice.start), slice.byteCount);
        for i in 0..<paddingCount {
            result.appendChar(char)
        }
        result
    }
}
