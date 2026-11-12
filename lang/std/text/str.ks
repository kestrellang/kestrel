// Read-only text API, on StringSlice and reached by String through its peel

module std.text

import std.core.(Bool, Equatable, Comparable, Ordering, Hashable, Hasher, fatalError)
import std.numeric.(Int64, UInt8)
import std.result.(Optional)
import std.memory.(Pointer, RawPointer)
import std.collections.(Array, Slice)
import std.iter.(Iterable)
import std.ffi.(memmem)
import std.text.(Formattable, Formatter, FormatOptions, Char, decodeUtf8, String, StringBuilder, StringSlice, BytesView, CharsView, GraphemesView, LinesView, WordsView, ByteIndex, CharIndex, GraphemeIndex, LineIndex, SplitView, SplitWhereView, _bytesEqual)
import std.text.unicode as unicode

// ============================================================================
// READ-ONLY TEXT METHODS
// ============================================================================

/// Every read-only text method, defined once on `StringSlice`. `String`
/// reaches them through its `Indirection` peel, and any parameter typed
/// `&StringSlice` takes a `String` or a string literal as well.
extend StringSlice {

    // -- Views ---------------------------------------------------------------

    /// View over the raw UTF-8 bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// "hi".bytes.count;  // 2
    /// ```
    public var bytes: BytesView { BytesView(slice: self) }

    /// View over Unicode code points.
    ///
    /// # Examples
    ///
    /// ```
    /// "caf\u{00E9}".chars.count;  // 4
    /// ```
    public var chars: CharsView { CharsView(slice: self) }

    /// View over grapheme clusters (user-perceived characters).
    ///
    /// # Examples
    ///
    /// ```
    /// "caf\u{00E9}".graphemes.count;  // 4
    /// ```
    public var graphemes: GraphemesView { GraphemesView(slice: self) }

    /// View over lines, recognising `\n`, `\r\n`, and `\r`.
    ///
    /// # Examples
    ///
    /// ```
    /// "a\nb\nc".lines.count;  // 3
    /// ```
    public var lines: LinesView { LinesView(slice: self) }

    /// View over the whitespace-separated words. Runs of whitespace act as
    /// one break and never yield empty words (unlike `split(by: " ")`).
    ///
    /// # Examples
    ///
    /// ```
    /// "  a  b\tc ".words.collect();  // ["a", "b", "c"]
    /// ```
    public var words: WordsView { WordsView(slice: self) }

    // -- Conversion ----------------------------------------------------------

    /// Copies this string's raw UTF-8 bytes into a new `Array[UInt8]`.
    ///
    /// Single bulk copy of the whole byte range (one buffer clone +
    /// one memcpy-style loop), so it is O(n) — unlike appending byte
    /// by byte, where each COW write re-clones the whole buffer and
    /// the total cost is O(n²).
    ///
    /// # Examples
    ///
    /// ```
    /// "Hi".toBytes().count;  // 2
    /// "Hi".toBytes()(0);     // 72  ('H')
    /// ```
    public func toBytes() -> Array[UInt8] {
        let n = self._len;
        var buffer = Array[UInt8](capacity: n);
        if n == 0 {
            return buffer
        }
        // asByteSlice() borrows the live string buffer;
        // append(contentsOf:) copies out of it in a single pass.
        buffer.append(contentsOf: self.asByteSlice());
        buffer
    }

    /// Returns a borrowed `Slice[UInt8]` view over this string's raw
    /// UTF-8 bytes — no copy, no allocation. The view borrows the live
    /// string buffer, so it can't outlive the string. O(1).
    ///
    /// Use this to hand the bytes to a sink (a socket, a hasher, an
    /// FFI call) without materializing an `Array[UInt8]` via `toBytes()`.
    ///
    /// # Examples
    ///
    /// ```
    /// let bytes = "Hi".asByteSlice();
    /// bytes.count;  // 2
    /// ```
    public func asByteSlice() -> &Slice[UInt8] {
        unsafe Slice.lend(start: self._rawPtr(), count: self._len)
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
    public func contains(substring: &StringSlice) -> Bool {
        self.firstIndex(of: substring).isSome
    }

    /// Returns true if any code point matches `predicate`.
    ///
    /// # Examples
    ///
    /// ```
    /// "abc123".contains(where: { (c) in c.isAsciiDigit });  // true
    /// ```
    public func contains(where predicate: (Char) -> Bool) -> Bool {
        self.chars.firstIndex(where: predicate).isSome
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
    public func starts(with prefix: &StringSlice) -> Bool {
        let prefixLen = prefix._len;
        if prefixLen > self._len { return false }
        if prefixLen == 0 { return true }
        _bytesEqual(a: self._rawPtr(), b: prefix._rawPtr(), n: prefixLen)
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
    public func ends(with suffix: &StringSlice) -> Bool {
        let suffixLen = suffix._len;
        if suffixLen > self._len { return false }
        if suffixLen == 0 { return true }
        _bytesEqual(a: self._rawPtr().offset(by: self._len - suffixLen), b: suffix._rawPtr(), n: suffixLen)
    }

    /// Returns the byte index of the first occurrence of `substring`,
    /// or `None` if not found.
    ///
    /// The index counts from this string's or slice's first byte, so it
    /// feeds straight into `subslice(from:to:)` and `bytes`. The empty
    /// substring matches at the start. Uses `memmem` for efficient
    /// byte-level search.
    ///
    /// # Examples
    ///
    /// ```
    /// "hello world".firstIndex(of: "world");  // Some(ByteIndex(6))
    /// "hello world".firstIndex(of: "xyz");    // None
    /// ```
    public func firstIndex(of substring: &StringSlice) -> ByteIndex? {
        let subLen = substring._len;
        let myLen = self._len;
        if subLen == 0 {
            return .Some(ByteIndex(0))
        }
        if subLen > myLen { return .None }
        let base = self._rawPtr().asRaw();
        let needle = substring._rawPtr().asRaw();
        let result = unsafe memmem(base, myLen, needle, subLen);
        if result.isNull {
            .None
        } else {
            let diff: lang.i64 = lang.i64_sub(result.address.raw, base.address.raw);
            .Some(ByteIndex(Int64(intLiteral: diff)))
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
    public func lastIndex(of substring: &StringSlice) -> ByteIndex? {
        let subLen = substring._len;
        let myLen = self._len;
        if subLen == 0 {
            return .Some(ByteIndex(myLen))
        }
        if subLen > myLen { return .None }
        let myPtr = self._rawPtr();
        let needlePtr = substring._rawPtr().asRaw();
        var lastFound: Int64 = -1;
        var i: Int64 = 0;
        while myLen - i >= subLen {
            let base = myPtr.offset(by: i).asRaw();
            let remaining = myLen - i;
            let result = unsafe memmem(base, remaining, needlePtr, subLen);
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
            .Some(ByteIndex(lastFound))
        }
    }

    // -- Splitting -------------------------------------------------------------

    /// Returns a lazy view that splits on `separator`, yielding
    /// zero-copy `StringSlice` segments.
    ///
    /// The empty separator is special-cased to split per code
    /// point. Adjacent separators produce empty segments, as does a
    /// separator at either end. An empty string has no segments.
    ///
    /// # Examples
    ///
    /// ```
    /// "a,b,c".split(by: ",").collect();   // ["a", "b", "c"]
    /// "a,,b".split(by: ",").count;        // 3 (empty segment preserved)
    /// "a,".split(by: ",").count;          // 2 (["a", ""])
    /// ```
    public func split(by separator: &StringSlice) -> SplitView {
        SplitView(slice: self, separator: separator, limit: 0)
    }

    /// Like `split(by:)`, but yields at most `limit` pieces: the last one
    /// holds the rest of the string, separators and all. `limit` must be
    /// at least 1.
    ///
    /// # Examples
    ///
    /// ```
    /// "a,b,c".split(by: ",", limit: 2).collect();  // ["a", "b,c"]
    /// "a,b,c".split(by: ",", limit: 1).collect();  // ["a,b,c"]
    /// ```
    public func split(by separator: &StringSlice, limit limit: Int64) -> SplitView {
        if limit < 1 {
            fatalError("split(by:limit:): limit must be at least 1")
        }
        SplitView(slice: self, separator: separator, limit: limit)
    }

    /// Splits at the first occurrence of `separator`, returning the parts
    /// before and after it, or `.None` when it doesn't occur.
    ///
    /// # Examples
    ///
    /// ```
    /// if let .Some((key, value)) = "port=8080".split(once: "=") {
    ///     // key == "port", value == "8080"
    /// }
    /// "no-equals".split(once: "=");  // .None
    /// ```
    public func split(once separator: &StringSlice) -> (&StringSlice, &StringSlice)? {
        match self.firstIndex(of: separator) {
            .Some(at) => {
                let head = self.subslice(from: 0, to: at.value);
                let tail = self.subslice(from: at.value + separator.bytes.count, to: self.bytes.count);
                .Some((head, tail))
            },
            .None => .None,
        }
    }

    /// Returns a lazy view that splits at every code point matching
    /// `predicate`, yielding zero-copy `StringSlice` segments.
    ///
    /// The matching characters are not included in any segment.
    ///
    /// # Examples
    ///
    /// ```
    /// "hello world".split(where: { (c) in c.isWhitespace }).count;  // 2
    /// ```
    public func split(consuming where predicate: escaping (Char) -> Bool) -> SplitWhereView {
        SplitWhereView(slice: self, predicate: predicate)
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
    public func trimmed() -> &StringSlice {
        let myLen = self._len;
        let basePtr = self._rawPtr();
        var realStart: Int64 = 0;
        var startDone: Bool = false;
        while realStart < myLen and startDone == false {
            let byte = unsafe basePtr.offset(by: realStart).read();
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
            let byte = unsafe basePtr.offset(by: idx).read();
            let v: lang.i32 = lang.cast_i8_i32(byte.raw);
            let isWs = lang.i1_or(lang.i1_or(lang.i1_or(lang.i1_or(lang.i32_eq(v, 32), lang.i32_eq(v, 9)), lang.i32_eq(v, 10)), lang.i32_eq(v, 13)), lang.i32_eq(v, 12));
            if Bool(boolLiteral: isWs) {
                endPos = endPos - 1
            } else {
                endDone = true
            }
        }
        self.subslice(from: realStart, to: endPos)
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
    public func trimmedStart() -> &StringSlice {
        let myLen = self._len;
        let basePtr = self._rawPtr();
        var realStart: Int64 = 0;
        var done: Bool = false;
        while realStart < myLen and done == false {
            let byte = unsafe basePtr.offset(by: realStart).read();
            let v: lang.i32 = lang.cast_i8_i32(byte.raw);
            let isWs = lang.i1_or(lang.i1_or(lang.i1_or(lang.i1_or(lang.i32_eq(v, 32), lang.i32_eq(v, 9)), lang.i32_eq(v, 10)), lang.i32_eq(v, 13)), lang.i32_eq(v, 12));
            if Bool(boolLiteral: isWs) {
                realStart = realStart + 1
            } else {
                done = true
            }
        }
        self.subslice(from: realStart, to: self._len)
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
    public func trimmedEnd() -> &StringSlice {
        let myLen = self._len;
        let basePtr = self._rawPtr();
        var endPos: Int64 = myLen;
        var done: Bool = false;
        while endPos > 0 and done == false {
            let idx = endPos - 1;
            let byte = unsafe basePtr.offset(by: idx).read();
            let v: lang.i32 = lang.cast_i8_i32(byte.raw);
            let isWs = lang.i1_or(lang.i1_or(lang.i1_or(lang.i1_or(lang.i32_eq(v, 32), lang.i32_eq(v, 9)), lang.i32_eq(v, 10)), lang.i32_eq(v, 13)), lang.i32_eq(v, 12));
            if Bool(boolLiteral: isWs) {
                endPos = endPos - 1
            } else {
                done = true
            }
        }
        self.subslice(from: 0, to: endPos)
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
    /// "00042".trimmed(where: { (c) in c.isEqual(to: '0') }).toOwned();  // "42"
    /// ```
    public func trimmed(where predicate: (Char) -> Bool) -> &StringSlice {
        let myLen = self._len;
        let basePtr = self._rawPtr();
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
        self.subslice(from: realStart, to: lastNonMatch)
    }

    /// Returns a zero-copy slice with leading code points matching
    /// `predicate` removed. Trailing matches are preserved.
    ///
    /// # Examples
    ///
    /// ```
    /// "000abc".trimmedStart(where: { (c) in c.isEqual(to: '0') }).toOwned();  // "abc"
    /// ```
    public func trimmedStart(where predicate: (Char) -> Bool) -> &StringSlice {
        let myLen = self._len;
        let basePtr = self._rawPtr();
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
        self.subslice(from: realStart, to: self._len)
    }

    /// Returns a zero-copy slice with trailing code points matching
    /// `predicate` removed. Leading matches are preserved.
    ///
    /// # Examples
    ///
    /// ```
    /// "abc000".trimmedEnd(where: { (c) in c.isEqual(to: '0') }).toOwned();  // "abc"
    /// ```
    public func trimmedEnd(where predicate: (Char) -> Bool) -> &StringSlice {
        let myLen = self._len;
        let basePtr = self._rawPtr();
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
        self.subslice(from: 0, to: lastNonMatch)
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
        let myLen = self._len;
        let basePtr = self._rawPtr();
        var hasUpperAscii = false;
        var i: Int64 = 0;
        while i < myLen {
            let byte = unsafe basePtr.offset(by: i).read();
            if byte > 127 {
                var result = String();
                for c in self.chars.iter() {
                    if unicode.hasLowercaseExpansion(c) {
                        result.append(unicode.lowercaseExpansion(c))
                    } else {
                        result.append(char: unicode.toLowercase(c))
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
            let byte = unsafe basePtr.offset(by: i).read();
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
        let myLen = self._len;
        let basePtr = self._rawPtr();
        var hasLowerAscii = false;
        var i: Int64 = 0;
        while i < myLen {
            let byte = unsafe basePtr.offset(by: i).read();
            if byte > 127 {
                var result = String();
                for c in self.chars.iter() {
                    if unicode.hasUppercaseExpansion(c) {
                        result.append(unicode.uppercaseExpansion(c))
                    } else {
                        result.append(char: unicode.toUppercase(c))
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
            let byte = unsafe basePtr.offset(by: i).read();
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
                result.append(char: c);
                atWordStart = true
            } else if atWordStart {
                if unicode.hasTitlecaseExpansion(c) {
                    result.append(unicode.titlecaseExpansion(c))
                } else {
                    result.append(char: unicode.toTitlecase(c))
                }
                atWordStart = false
            } else {
                if unicode.hasLowercaseExpansion(c) {
                    result.append(unicode.lowercaseExpansion(c))
                } else {
                    result.append(char: unicode.toLowercase(c))
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
        let myLen = self._len;
        let basePtr = self._rawPtr();
        var result = String(capacity: myLen);
        var i: Int64 = 0;
        while i < myLen {
            let byte = unsafe basePtr.offset(by: i).read();
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
        let myLen = self._len;
        let basePtr = self._rawPtr();
        var result = String(capacity: myLen);
        var i: Int64 = 0;
        while i < myLen {
            let byte = unsafe basePtr.offset(by: i).read();
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
    public func equalsCaseInsensitive(other: &StringSlice) -> Bool {
        self.caseFolded().isEqual(to: other.caseFolded())
    }

    /// ASCII case-insensitive equality — folds only `A`–`Z` ↔ `a`–`z`;
    /// bytes `>= 0x80` must match exactly.
    ///
    /// Allocates nothing: it compares the raw UTF-8 bytes in place behind a
    /// length fast-path, so it is far cheaper than `equalsCaseInsensitive`,
    /// which builds two Unicode-folded `String`s. Use it for ASCII tokens
    /// such as HTTP header names, where Unicode folding is unnecessary.
    ///
    /// # Examples
    ///
    /// ```
    /// "Content-Type".equalsCaseInsensitiveAscii("content-type");  // true
    /// "Content-Type".equalsCaseInsensitiveAscii("Host");          // false
    /// ```
    public func equalsCaseInsensitiveAscii(other: &StringSlice) -> Bool {
        let a = self.asByteSlice();
        let b = other.asByteSlice();
        if a.count != b.count {
            return false
        }
        var i: Int64 = 0;
        while i < a.count {
            var x = unsafe a.pointer.offset(by: i).read();
            var y = unsafe b.pointer.offset(by: i).read();
            // Fold A–Z (0x41–0x5A) to lower case; leave everything else.
            if x >= 65 and x <= 90 { x = x + 32 };
            if y >= 65 and y <= 90 { y = y + 32 };
            if x != y {
                return false
            };
            i = i + 1
        };
        true
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
        var result = String(capacity: self.bytes.count);
        for c in self.chars {
            result.append(char: unicode.caseFold(c))
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
    public func replaced(pattern: &StringSlice, with replacement: &StringSlice) -> String {
        let patternLen = pattern._len;
        if patternLen == 0 {
            return self.toOwned()
        }
        let myLen = self._len;
        if patternLen > myLen {
            return self.toOwned()
        }
        let myPtr = self._rawPtr();
        let patternPtr = pattern._rawPtr();

        var matchCount: Int64 = 0;
        var i: Int64 = 0;
        while myLen - i >= patternLen {
            let base = myPtr.offset(by: i).asRaw();
            let r = unsafe memmem(base, myLen - i, patternPtr.asRaw(), patternLen);
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
        let repLen = replacement._len;
        let repPtr = replacement._rawPtr();
        let resultLen = myLen - matchCount * patternLen + matchCount * repLen;
        var result = String(capacity: resultLen);
        var runStart: Int64 = 0;
        i = 0;
        while myLen - i >= patternLen {
            let base = myPtr.offset(by: i).asRaw();
            let r = unsafe memmem(base, myLen - i, patternPtr.asRaw(), patternLen);
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
        let myLen = self._len;
        if myLen == 0 {
            return String()
        }
        let basePtr = self._rawPtr();
        let totalLen = myLen * count;
        var result = String(capacity: totalLen);
        for i in 0..<count {
            result._appendBytes(basePtr, myLen)
        }
        result
    }

    /// Returns the string padded with `fill` to at least `width` graphemes.
    ///
    /// `side` says where the padding goes: `.Trailing` (the default) after
    /// the text, `.Leading` before it, or `.Center` on both sides, with the
    /// extra one after when the padding is odd. Width counts graphemes, so
    /// `"é"` is one wide whether it is written as one code point or two. A
    /// string that is already wide enough is returned unchanged.
    ///
    /// # Examples
    ///
    /// ```
    /// "ab".padded(toWidth: 5);                           // "ab   "
    /// "42".padded(toWidth: 5, with: '0', side: .Leading); // "00042"
    /// "ab".padded(toWidth: 7, with: '*', side: .Center);  // "**ab***"
    /// "hello".padded(toWidth: 3);                        // "hello"
    /// ```
    public func padded(toWidth width: Int64, with fill: Char = ' ', side side: PaddingSide = .Trailing) -> String {
        let padding = width - self.graphemes.count;
        let byteCount = self._len;
        if padding <= 0 {
            return self.toOwned()
        }
        let before = match side {
            .Leading => padding,
            .Trailing => 0,
            .Center => padding / 2,
        };
        var result = String(capacity: byteCount + padding * fill.utf8Length());
        for _ in 0..<before {
            result.append(char: fill)
        }
        result._appendBytes(self._rawPtr(), byteCount);
        for _ in 0..<(padding - before) {
            result.append(char: fill)
        }
        result
    }
}

/// Where `padded(toWidth:with:side:)` puts the padding.
@derive(Equatable)
public enum PaddingSide {
    /// Before the text, so it lines up on the right: `"   42"`.
    case Leading
    /// After the text, so it lines up on the left: `"42   "`.
    case Trailing
    /// On both sides; the extra one goes after when the padding is odd.
    case Center
}
