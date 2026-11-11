// String - UTF-8 encoded string with COW (Copy-on-Write) semantics

module std.text

import std.core.(Bool, Equal, NotEqual, Equatable, Matchable, Comparable, Cloneable, Ordering, Addable, ExpressibleByStringLiteral, Hashable, Hasher, Defaultable, Indirection, fatalError)
import std.text.(Formattable, Formatter, StringBuilder, _writePadded)
import std.numeric.(Int64, UInt8)
import std.result.(Optional)
import std.memory.(Allocator, GlobalAllocator, Layout, Pointer, RawPointer, RcBox, CowBox, UnsafeSlice)
import std.iter.(Iterator, Iterable)
import std.collections.(Slice)
import std.numeric.(UInt32)
import std.text.(Char, decodeUtf8, encodeUtf8, BytesView, CharsView, CharsIterator, GraphemesView, LinesView, CharsSubstringIndex, StringSlice, SplitView, SplitWhereView)
import std.text.unicode as unicode
import std.ffi.(memcpy, memmove, memcmp, memmem)

// ============================================================================
// INTERNAL HELPERS
// ============================================================================

/// Bulk byte copy via libc `memcpy`. Caller ensures the regions are
/// disjoint and each is valid for `n` bytes; `n <= 0` is a no-op.
func _memcpyBytes(dst dst: Pointer[UInt8], src src: Pointer[UInt8], n n: Int64) {
    if n <= 0 {
        return
    }
     unsafe memcpy(dst.asRaw(), src.asRaw(), n);
}

/// Byte-wise equality of two regions via libc `memcmp`. Caller ensures
/// each region is valid for `n` bytes. `n <= 0` reports equal.
func _bytesEqual(a a: Pointer[UInt8], b b: Pointer[UInt8], n n: Int64) -> Bool {
    if n <= 0 {
        return true
    }
    unsafe memcmp(a.asRaw(), b.asRaw(), n) == 0
}

/// Allocates a typed `Pointer[UInt8]` of `layout` bytes from `allocator`,
/// panicking on failure. Centralizes the allocate/cast/panic boilerplate
/// that every constructor would otherwise duplicate.
func _textAlloc[A](layout: Layout, from allocator: A) -> Pointer[UInt8] where A: Allocator {
    var a = allocator;
    match a.allocate(layout) {
        .Some(p) => p.cast[UInt8](),
        .None => fatalError("String allocation failed")
    }
}

/// Gives a buffer previously returned by `_textAlloc` back to `allocator`.
func _textDealloc[A](ptr: Pointer[UInt8], layout: Layout, to allocator: A) where A: Allocator {
    var a = allocator;
    unsafe a.deallocate(ptr.asRaw(), layout)
}

/// Lexicographic byte-wise comparison via libc `memcmp`. Returns the
/// `Ordering` for the first `n` bytes of the two regions.
func _bytesCompare(a a: Pointer[UInt8], b b: Pointer[UInt8], n n: Int64) -> Ordering {
    if n <= 0 {
        return .Equal
    }
    let r = unsafe memcmp(a.asRaw(), b.asRaw(), n);
    if r < 0 {
        .Less
    } else if r > 0 {
        .Greater
    } else {
        .Equal
    }
}

// ============================================================================
// STRING STORAGE (Internal)
// ============================================================================

/// The bytes of a `String`: what a `StringSlice` borrows.
///
/// `cap == 0` means the bytes are not owned: the empty string (null
/// pointer, `len == 0`) or a string literal borrowing its static bytes
/// (`len > 0`). Such bytes are read-only; every write path goes through
/// `String.grow`, which copies them into an owned buffer first.
///
/// # Representation
///
/// `(ptr: Pointer[UInt8], len: Int64, cap: Int64)`. Plain data: the
/// `StringStorage` holding it owns the buffer.
struct StringBytes {
    var ptr: Pointer[UInt8]
    var len: Int64
    var cap: Int64
}

/// Internal heap buffer for `String` — the value sitting inside the `CowBox`.
///
/// Owns the buffer `bytes` points at, and the allocator it came from.
/// Cloning allocates a fresh buffer from that allocator (used by COW);
/// the deinit gives it back if `cap > 0`.
///
/// # Representation
///
/// `(bytes: StringBytes, allocator: A)`.
struct StringStorage[A]: Cloneable where A: Allocator {
    var bytes: StringBytes
    var allocator: A

    /// @name From Fields
    /// Constructs a storage record from a pointer, length, and capacity.
    ///
    /// Internal: callers must ensure `ptr` is either a fresh allocation
    /// of `cap` bytes from `allocator` (with `len <= cap`), static bytes
    /// with `cap == 0`, or a null pointer with both counts zero.
    init(ptr ptr: Pointer[UInt8], len len: Int64, cap cap: Int64, allocator allocator: A) {
        self.bytes = StringBytes(ptr: ptr, len: len, cap: cap);
        self.allocator = allocator;
    }

    /// Allocates a new buffer and copies the bytes.
    ///
    /// Used when COW detects shared storage and a mutation is about to
    /// happen. Preserves an allocated buffer even when `len == 0` so
    /// that a grow → write sequence doesn't lose the buffer through a
    /// clone chain (CopyValue on Named types expands to clone calls).
    func clone() -> StringStorage[A] {
        let b = self.bytes;
        // `cap == 0` owns nothing: either the empty string or a literal's
        // static bytes. Both are shared as-is.
        if b.cap == 0 {
            return StringStorage(ptr: b.ptr, len: b.len, cap: 0, allocator: self.allocator)
        }
        var allocator = self.allocator;
        let newPtr = _textAlloc(Layout.array[UInt8](b.cap), from: allocator);
        _memcpyBytes(dst: newPtr, src: b.ptr, n: b.len);
        StringStorage(ptr: newPtr, len: b.len, cap: b.cap, allocator: allocator)
    }

    /// Frees the buffer if any was allocated.
    deinit {
        if self.bytes.cap > 0 {
            _textDealloc(self.bytes.ptr, Layout.array[UInt8](self.bytes.cap), to: self.allocator)
        }
    }
}

// ============================================================================
// STRING
// ============================================================================

/// A UTF-8 encoded, dynamically sized string with copy-on-write semantics.
///
/// `String` is the standard text type. The bytes are always valid
/// UTF-8. Storage is shared between clones via an
/// `RcBox`; mutating a `String` whose storage is referenced elsewhere
/// triggers a copy. Three different views (`bytes`, `chars`,
/// `graphemes`) plus a `lines` view expose different units of
/// iteration over the same buffer.
///
/// # Examples
///
/// ```
/// var s = "hello";
/// s.append(", world");
/// s.bytes.count;          // 12
/// s.contains(",");  // true
/// for line in "a\nb".lines { /* ... */ }
/// ```
///
/// # UTF-8
///
/// All public mutators preserve UTF-8 validity. The `bytes` view
/// returns raw `UInt8`s for hashing and FFI; the `chars` view decodes
/// code points; the `graphemes` view applies UAX #29 segmentation for
/// user-perceived characters. Choose the view that matches your unit:
/// byte-level work uses `bytes`, scalar-level work uses `chars`, and
/// anything user-visible (cursor movement, truncation) uses `graphemes`.
///
/// # Representation
///
/// A single `CowBox[StringStorage]` field. The storage record carries
/// `(ptr, len, cap)`; the empty string uses a null pointer with both
/// counts zero.
///
/// # Memory Model
///
/// Reference-counted, copy-on-write. Cloning is O(1); the first
/// mutation after a shared clone allocates and copies the bytes. The
/// raw byte pointer returned from `bytes` aliases the live buffer;
/// retain strings, not pointers.
///
/// # Guarantees
///
/// - Bytes are valid UTF-8 after every public mutator.
/// - `bytes.count`, `capacity`, and `isEmpty` are O(1); `chars.count` (code
///   points) is O(n).
/// - Clones do not share mutation; `s.clone()` and `s` will diverge as
///   soon as either is mutated.
public struct String[A = GlobalAllocator]: Iterable, Equatable, Equal[String[A]], Equal[StringSlice], NotEqual[String[A]], NotEqual[StringSlice], Matchable, Comparable, Cloneable, Formattable, Hashable where A: Allocator {
    /// The element type yielded by iteration — always `Char`.
    type Item = Char
    /// The iterator type returned by `iter()`.
    type TargetIterator = CharsIterator
    type Equal[String[A]].Output = Bool
    type Equal[StringSlice].Output = Bool
    type NotEqual[String[A]].Output = Bool
    type NotEqual[StringSlice].Output = Bool

    fileprivate var storage: CowBox[StringStorage[A]]

    // Helper accessors for storage fields
    fileprivate func ptr() -> Pointer[UInt8] { self.storage.value.bytes.ptr }
    fileprivate func len() -> Int64 { self.storage.value.bytes.len }
    fileprivate func cap() -> Int64 { self.storage.value.bytes.cap }

    // ========================================================================
    // CONSTRUCTORS (explicit allocator)
    // ========================================================================

    /// @name Empty With Allocator
    /// An empty string whose buffer will come from `allocator`. Nothing is
    /// allocated until the first write.
    ///
    /// # Examples
    ///
    /// ```
    /// var s = String[ArenaAllocator](allocator: arena);
    /// s.append("hi");  // the bytes live in `arena`
    /// ```
    public init(allocator allocator: A) {
        self.storage = CowBox(StringStorage(
            ptr: Pointer[UInt8].nullPointer(),
            len: 0,
            cap: 0,
            allocator: allocator
        ));
    }

    /// @name With Capacity and Allocator
    /// An empty string with at least `capacity` bytes preallocated from
    /// `allocator`. A non-positive `capacity` allocates nothing.
    ///
    /// # Errors
    ///
    /// Panics with `"String allocation failed"` if the allocator returns
    /// null.
    public init(capacity capacity: Int64, allocator allocator: A) {
        if capacity <= 0 {
            self.init(allocator: allocator)
        } else {
            self.storage = CowBox(StringStorage(
                ptr: _textAlloc(Layout.array[UInt8](capacity), from: allocator),
                len: 0,
                cap: capacity,
                allocator: allocator
            ))
        }
    }

    /// @name Copying Bytes
    /// Copies `count` bytes from `ptr` into a buffer from `allocator`,
    /// without UTF-8 validation. Module-internal: the one place a string
    /// is built from a byte range.
    init(copying ptr: Pointer[UInt8], count count: Int64, allocator allocator: A) {
        if count <= 0 {
            self.init(allocator: allocator)
        } else {
            let newPtr = _textAlloc(Layout.array[UInt8](count), from: allocator);
            _memcpyBytes(dst: newPtr, src: ptr, n: count);
            self.storage = CowBox(StringStorage(ptr: newPtr, len: count, cap: count, allocator: allocator))
        }
    }

    /// @name From Storage
    /// Wraps an existing `CowBox[StringStorage[A]]` as a new `String`.
    ///
    /// Module-internal — used by `clone()`, `StringBuilder.build()`,
    /// and other std.text code that constructs strings from raw storage.
    init(storage storage: CowBox[StringStorage[A]]) {
        self.storage = storage;
    }

    /// The allocator this string's buffer comes from.
    public var allocator: A { self.storage.value.allocator }

    // ========================================================================
    // VIEW PROPERTIES
    // ========================================================================

    /// `s.bytes` — view over the raw UTF-8 bytes. O(1) byte indexing,
    /// byte-level iteration. Index via the view's subscripts:
    /// `s.bytes(i)`, `s.bytes(checked: i)`, `s.bytes(0..<n)`.
    public var bytes: BytesView { BytesView(slice: self.asSlice()) }

    /// `s.chars` — view over the Unicode code points. O(n) indexing,
    /// scalar-level iteration. Index via the view's subscripts:
    /// `s.chars(i)`, `s.chars(checked: i)`.
    public var chars: CharsView { CharsView(slice: self.asSlice()) }

    /// `s.graphemes` — view over user-perceived characters
    /// (UAX #29 grapheme clusters). Iterate or count, no random access.
    public var graphemes: GraphemesView { GraphemesView(slice: self.asSlice()) }

    /// A view over the lines of the string, recognising `\n`, `\r\n`, and `\r`.
    public var lines: LinesView { LinesView(slice: self.asSlice()) }

    // ========================================================================
    // SLICE VIEW
    // ========================================================================

    /// Lends the whole string as a `StringSlice`, in place: zero-copy, no
    /// refcount. Valid until the string next changes or is dropped. Every
    /// read-only text method is on `StringSlice`; `String` reaches them
    /// through its `Indirection` peel.
    public func asSlice() -> &StringSlice {
        unsafe StringSlice.lend(start: self.ptr(), count: self.len())
    }

    // ========================================================================
    // SIZE & CAPACITY
    // ========================================================================

    /// The number of bytes the storage buffer can hold without reallocating. O(1).
    public var capacity: Int64 { self.cap() }

    /// True if the string holds zero bytes. O(1).
    public var isEmpty: Bool { self.len() == 0 }

    // ========================================================================
    // CAPACITY MANAGEMENT (Internal)
    // ========================================================================

    /// Grows the buffer to at least `minCapacity` bytes, preserving content.
    ///
    /// Geometric-growth (doubles each step from a base of 16) so an N
    /// append loop stays amortised O(N). Ensures unique ownership
    /// via `write()` before reallocating.
    private mutating func grow(minCapacity: Int64) {
        let myCap = self.cap();
        if myCap >= minCapacity {
            return
        }

        var newCap: Int64 = myCap;
        if newCap == 0 {
            newCap = 16
        }
        while newCap < minCapacity {
            newCap = newCap * 2
        }

        let newLayout = Layout.array[UInt8](newCap);
        let newPtr = _textAlloc(newLayout, from: self.allocator);
        if not self.storage.isUnique {
            // Shared: copy straight into the bigger buffer. Writing through
            // `value` would fork an exact-size copy first, then copy again.
            let myLen = self.len();
            _memcpyBytes(dst: newPtr, src: self.ptr(), n: myLen);
            self.storage = CowBox(StringStorage(ptr: newPtr, len: myLen, cap: newCap, allocator: self.allocator));
            return
        }
        let allocator = self.allocator;
        let s = &mutating self.storage.value.bytes;
        _memcpyBytes(dst: newPtr, src: s.ptr, n: s.len);
        if s.cap > 0 {
            _textDealloc(s.ptr, Layout.array[UInt8](s.cap), to: allocator)
        }
        s.ptr = newPtr;
        s.cap = newCap
    }

    // ========================================================================
    // APPENDING
    // ========================================================================

    /// Appends `other`'s bytes to this string. COW.
    ///
    /// Triggers a copy if storage is shared. Empty appends are a fast
    /// no-op.
    ///
    /// # Examples
    ///
    /// ```
    /// var s = "hello";
    /// s.append(", world");
    /// s;  // "hello, world"
    /// ```
    public mutating func append(other: &StringSlice) {
        let otherLen = other._len;
        if otherLen == 0 {
            return
        }
        let myLen = self.len();
        // `other` may lend this string's own bytes (`s.append(s.trimmed())`).
        // Growing frees that buffer, so find them again in the new one.
        let src = other._rawPtr().address;
        let mine = self.ptr().address;
        let inSelf = myLen > 0 and src >= mine and src < mine + UInt64(truncating: myLen);
        self.grow(myLen + otherLen);
        let srcPtr = if inSelf { self.ptr().offset(by: Int64(truncating: src - mine)) } else { other._rawPtr() };
        let s = &mutating self.storage.value.bytes;
        _memcpyBytes(dst: s.ptr.offset(by: s.len), src: srcPtr, n: otherLen);
        s.len = s.len + otherLen
    }

    /// Appends a single code point, encoding it as UTF-8.
    ///
    /// Sizes the buffer for the encoded length (1–4 bytes) before
    /// writing.
    ///
    /// # Examples
    ///
    /// ```
    /// var s = "h";
    /// s.append(char: 'i');
    /// s.append(char: '\u{1F600}');
    /// s;  // "hi😀"
    /// ```
    public mutating func append(char c: Char) {
        let utf8Len = c.utf8Length();
        self.grow(self.len() + utf8Len);
        let s = &mutating self.storage.value.bytes;
        // Encode to buffer
        let rawPtr: lang.ptr[lang.i8] = lang.cast_ptr[_, lang.i8](s.ptr.asRaw().raw);
        let written = encodeUtf8(c, rawPtr, at: s.len);
        s.len = s.len + written
    }

    /// Appends a raw byte. Internal — caller ensures UTF-8 validity.
    ///
    /// Do not use to append ASCII characters: prefer `appendChar(c)` or
    /// `append(other)`. This exists only for low-level UTF-8 plumbing
    /// inside the stdlib (e.g. an encoder that already produced bytes).
    internal mutating func appendByte(byte: UInt8) {
        self.grow(self.len() + 1);
        let s = &mutating self.storage.value.bytes;
        unsafe s.ptr.offset(by: s.len).write(byte);
        s.len = s.len + 1
    }

    /// Appends `n` bytes from `ptr` via `memcpy`. Internal — caller
    /// ensures the bytes preserve UTF-8 validity.
    ///
    /// # Safety
    ///
    /// `ptr` must reference at least `n` valid UTF-8 bytes that, when
    /// concatenated to the current buffer, yield valid UTF-8.
    mutating func _appendBytes(ptr: Pointer[UInt8], n: Int64) {
        if n <= 0 {
            return
        }
        self.grow(self.len() + n);
        let s = &mutating self.storage.value.bytes;
        _memcpyBytes(dst: s.ptr.offset(by: s.len), src: ptr, n: n);
        s.len = s.len + n
    }

    /// Internal substring by byte range. Returns empty for invalid ranges.
    ///
    /// Do not use for per-character slicing in a loop — each call copies
    /// `end - start` bytes, so walking the string yields O(N²) behaviour.
    /// For iteration, use `decodeUtf8` with a running byte offset, or the
    /// `chars()` / `bytes()` views.
    internal func substringBytes(from start: Int64, to end: Int64) -> String[A] {
        let myLen = self.len();
        if start >= end or start < 0 or end > myLen {
            return String[A](allocator: self.allocator)
        }
        String[A](copying: self.ptr().offset(by: start), count: end - start, allocator: self.allocator)
    }

    /// Truncates the string to length zero, keeping the allocated buffer.
    ///
    /// Capacity is unchanged, so this is the right primitive for
    /// reusing a buffer in a hot loop.
    public mutating func clear() {
        // Shared storage: start fresh instead of forking a copy to discard.
        if not self.storage.isUnique {
            self = String[A](allocator: self.allocator);
            return
        }
        self.storage.value.bytes.len = 0
    }

    // ========================================================================
    // TRIMMING (Mutating)
    // ========================================================================

    /// Removes leading and trailing ASCII whitespace in place.
    ///
    /// Recognises the same whitespace set as `Char.isWhitespace`:
    /// space, tab, LF, CR, form feed. For Unicode-aware trimming, use
    /// the `(where:)` overloads with a custom predicate. Non-mutating
    /// mirrors live under `trimmed*`.
    ///
    /// # Examples
    ///
    /// ```
    /// var s = "  hi  ";
    /// s.trim();
    /// s;  // "hi"
    /// ```
    public mutating func trim() {
        // End first, so the start trim moves fewer bytes.
        self.trimEnd();
        self.trimStart()
    }

    /// Removes leading ASCII whitespace in place.
    public mutating func trimStart() {
        let start = self.len() - self.trimmedStart()._len;
        self.dropLeadingBytes(start)
    }

    /// Removes trailing ASCII whitespace in place.
    public mutating func trimEnd() {
        let end = self.trimmedEnd()._len;
        self.truncateBytes(end)
    }

    // Keeps the first `n` bytes (a char boundary). Shortening never needs
    // a new buffer, so this only forks when the storage is shared.
    private mutating func truncateBytes(n: Int64) {
        if n < self.len() {
            self.storage.value.bytes.len = n
        }
    }

    // Drops the first `n` bytes (a char boundary). An owned, unique buffer
    // slides its tail down in place; otherwise the tail is copied out.
    private mutating func dropLeadingBytes(n: Int64) {
        if n <= 0 {
            return
        }
        let myLen = self.len();
        if n >= myLen {
            self.clear();
            return
        }
        if self.cap() == 0 or not self.storage.isUnique {
            self = self.substringBytes(from: n, to: myLen);
            return
        }
        let s = &mutating self.storage.value.bytes;
        let _ = unsafe memmove(s.ptr.asRaw(), s.ptr.offset(by: n).asRaw(), myLen - n);
        s.len = myLen - n
    }

    /// Removes leading and trailing code points matching `predicate`, in place.
    ///
    /// # Examples
    ///
    /// ```
    /// var s = "***hi***";
    /// s.trim { (c) in c == '*' };
    /// s;  // "hi"
    /// ```
    public mutating func trim(where predicate: (Char) -> Bool) {
        let end = self.trimmedEnd(where: predicate)._len;
        self.truncateBytes(end);
        let start = self.len() - self.trimmedStart(where: predicate)._len;
        self.dropLeadingBytes(start)
    }

    /// Removes leading code points matching `predicate`, in place.
    public mutating func trimStart(where predicate: (Char) -> Bool) {
        let start = self.len() - self.trimmedStart(where: predicate)._len;
        self.dropLeadingBytes(start)
    }

    /// Removes trailing code points matching `predicate`, in place.
    public mutating func trimEnd(where predicate: (Char) -> Bool) {
        let end = self.trimmedEnd(where: predicate)._len;
        self.truncateBytes(end)
    }

    // ========================================================================
    // CASE CONVERSION (ASCII-only, mutating)
    // ========================================================================

    /// Lowercases ASCII letters in place; non-ASCII bytes are left untouched.
    ///
    /// Cheap byte-level scan with no Unicode tables. For locale-
    /// independent Unicode case folding, use `lowercase`.
    ///
    /// # Examples
    ///
    /// ```
    /// var s = "HéLLO";
    /// s.lowercaseAscii();
    /// s;  // "héllo" — only ASCII letters touched
    /// ```
    public mutating func lowercaseAscii() {
        let myLen = self.len();
        // A literal borrows read-only bytes; take a writable copy first.
        self.grow(myLen);
        let s = &mutating self.storage.value.bytes;
        for i in 0..<myLen {
            let byte = unsafe s.ptr.offset(by: i).read();
            let v: lang.i32 = lang.cast_i8_i32(byte.raw);
            // A-Z: 65-90 -> a-z: 97-122
            let isUppercase = lang.i1_and(lang.i32_signed_ge(v, 65), lang.i32_signed_le(v, 90));
            if Bool(boolLiteral: isUppercase) {
                unsafe s.ptr.offset(by: i).write(UInt8(raw: lang.cast_i32_i8(lang.i32_add(v, 32))))
            }
        }
    }

    /// Uppercases ASCII letters in place; non-ASCII bytes are left untouched.
    public mutating func uppercaseAscii() {
        let myLen = self.len();
        // A literal borrows read-only bytes; take a writable copy first.
        self.grow(myLen);
        let s = &mutating self.storage.value.bytes;
        for i in 0..<myLen {
            let byte = unsafe s.ptr.offset(by: i).read();
            let v: lang.i32 = lang.cast_i8_i32(byte.raw);
            // a-z: 97-122 -> A-Z: 65-90
            let isLowercase = lang.i1_and(lang.i32_signed_ge(v, 97), lang.i32_signed_le(v, 122));
            if Bool(boolLiteral: isLowercase) {
                unsafe s.ptr.offset(by: i).write(UInt8(raw: lang.cast_i32_i8(lang.i32_sub(v, 32))))
            }
        }
    }

    // ========================================================================
    // CASE CONVERSION (Unicode, mutating)
    // ========================================================================

    /// Replaces this string with its lowercase form using full Unicode case mapping.
    public mutating func lowercase() {
        // ASCII-only text maps byte for byte: convert in place.
        if self.isAsciiBytes() {
            self.lowercaseAscii();
            return
        }
        self.setContents(self.lowercased())
    }

    /// Replaces this string with its uppercase form using full Unicode case mapping.
    ///
    /// Locale-independent. Handles multi-character expansions —
    /// e.g. German `ß` → `SS`.
    public mutating func uppercase() {
        if self.isAsciiBytes() {
            self.uppercaseAscii();
            return
        }
        self.setContents(self.uppercased())
    }

    // Replaces the contents with `text`, keeping this string's allocator
    // (and its buffer, when unique and big enough).
    private mutating func setContents(text: String) {
        self.clear();
        self.append(text)
    }

    // True when every byte is below 0x80, so ASCII case mapping is exact.
    private func isAsciiBytes() -> Bool {
        let p = self.ptr();
        for i in 0..<self.len() {
            if unsafe p.offset(by: i).read() > 127 {
                return false
            }
        }
        true
    }

    // ========================================================================
    // REPLACEMENT (Mutating)
    // ========================================================================

    /// Replaces every occurrence of `pattern` with `replacement`, in place.
    public mutating func replace(pattern: &StringSlice, with replacement: &StringSlice) {
        // No match (or an empty pattern) leaves the string, and its buffer, alone.
        if pattern.isEmpty or not self.contains(pattern) {
            return
        }
        self.setContents(self.replaced(pattern, with: replacement))
    }

    // ========================================================================
    // ITERATION
    // ========================================================================

    /// Returns a `CharsIterator` over the code points starting at byte 0.
    ///
    /// Required by `Iterable`. Each call returns a fresh iterator;
    /// the string itself is reusable.
    public func iter() -> CharsIterator {
        CharsIterator(source: self.asSlice(), length: self.len(), byteIndex: 0)
    }

    // ========================================================================
    // PROTOCOL CONFORMANCES
    // ========================================================================

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
    public func isEqual(to other: String[A]) -> Bool {
        let myLen = self.len();
        let otherLen = other.len();
        if myLen != otherLen {
            return false
        }
        _bytesEqual(a: self.ptr(), b: other.ptr(), n: myLen)
    }

    public func equal(to other: String[A]) -> Bool {
        self.isEqual(to: other)
    }

    public func notEqual(to other: String[A]) -> Bool {
        if self.isEqual(to: other) { false } else { true }
    }

    public func equal(to other: StringSlice) -> Bool {
        self.asSlice().isEqual(to: other)
    }

    public func notEqual(to other: StringSlice) -> Bool {
        if self.asSlice().isEqual(to: other) { false } else { true }
    }

    /// Pattern-match form of `isEqual`: each `"literal" =>` arm
    /// dispatches through here, one `O(len)` comparison per arm tried.
    public func matches(other: String[A]) -> Bool {
        self.isEqual(to: other)
    }

    /// Lexicographic byte-wise comparison.
    ///
    /// Returns `Less` / `Equal` / `Greater` according to the first
    /// differing byte; if one string is a prefix of the other, the
    /// shorter is less. Byte order coincides with code-point order
    /// because UTF-8 is order-preserving — this is *not* the same as
    /// locale-aware collation.
    ///
    /// # Examples
    ///
    /// ```
    /// "abc".compare("abd");  // Less
    /// "abc".compare("ab");   // Greater
    /// "abc".compare("abc");  // Equal
    /// ```
    public func compare(other: String[A]) -> Ordering {
        let myLen = self.len();
        let otherLen = other.len();
        var minLen: Int64 = myLen;
        if otherLen < minLen {
            minLen = otherLen
        }
        let cmp = _bytesCompare(a: self.ptr(), b: other.ptr(), n: minLen);
        let eql: Ordering = .Equal;
        if cmp.isEqual(to: eql) == false {
            return cmp
        }
        if myLen < otherLen {
            .Less
        } else if myLen > otherLen {
            .Greater
        } else {
            .Equal
        }
    }

    /// Hashes the raw byte sequence into the supplied hasher.
    ///
    /// Sends the whole buffer in a single `write` so the hasher gets
    /// to choose how to consume it.
    public func hash[H](mutating into hasher: H) where H: Hasher {
        hasher.write(unsafe Slice.lend(start: self.ptr(), count: self.len()));
        // The length ends the string, so ["ab", "c"] and ["a", "bc"]
        // feed the hasher different input. Must match `StringSlice.hash`.
        hasher.write(uint64: UInt64(truncating: self.len()))
    }

    /// Returns a shallow clone — storage is shared until either side mutates.
    ///
    /// O(1). Mutation triggers a deep copy via `CowBox.write()`.
    public func clone() -> String[A] {
        String(storage: self.storage.clone())
    }

    /// Renders this string under the supplied `FormatOptions`.
    ///
    /// Honours `width`, `alignment`, and `fill`. `precision` /
    /// `radix` / `floatStyle` / `sign` are ignored — they don't apply
    /// to strings. Aligned padding is measured in *code points*, not
    /// bytes, so multi-byte characters count as one column for
    /// alignment purposes (display width still depends on font).
    ///
    /// # Examples
    ///
    /// ```
    /// var opts = FormatOptions();
    /// opts.width = .Some(10);
    /// opts.alignment = .Left;
    /// "test".formatted(opts);   // "test      "
    /// opts.alignment = .Right;
    /// "test".formatted(opts);   // "      test"
    /// opts.alignment = .Center;
    /// "test".formatted(opts);   // "   test   "
    /// ```
    public func format(mutating into writer: some Formatter, options: FormatOptions = FormatOptions.default()) {
        _writePadded(into: writer, self, options)
    }
}

// Generic over the allocator like `init(from: CString)`, so the two
// `init(from:)` overloads are ranked together and picked by argument type.
extend String[A] where A: Defaultable {
    /// @name From Char Iterable
    /// Builds a string by encoding each character of `chars` as UTF-8.
    ///
    /// Mirrors `Array.init(from:)` and `Set.init(from:)` — accepts any
    /// `Iterable` whose `Item` is `Char`. Useful for materializing the
    /// result of an iterator chain back into a `String`:
    ///
    /// ```
    /// let upper = String(from: "hello".chars.iter().map { it.uppercased() }.collect());
    /// // "HELLO"
    /// ```
    public init[I](from chars: I) where I: Iterable, I: not Static, I.Item = Char {
        var b = StringBuilder[A]();
        var iter = chars.iter();
        while let .Some(c) = iter.next() {
            b.append(char: c)
        }
        self = b.build();
    }
}

// Constructors that make their own allocator, and the conformances built
// on them: a literal, `String()` and `zero` have no allocator to take.
extend String[A]: Defaultable, ExpressibleByStringLiteral, Addable where A: Defaultable {
    /// The output type of `+` (concatenation) — always `String`.
    type Output = String[A]

    /// The additive identity for strings — the empty string `""`.
    public static var zero: String[A] { get { "" } }

    /// Returns the concatenation `self + other`. Required by `Addable`.
    /// When `self` is uniquely owned (refcount 1), appends in place —
    /// no allocation. Otherwise builds a fresh string with both halves.
    public consuming func add(consuming other: String[A]) -> String[A] {
        if self.storage.isUnique {
            var result = self;
            result.append(other);
            result
        } else {
            var result = String[A](capacity: self.len() + other.len(), allocator: self.allocator);
            result.append(self);
            result.append(other);
            result
        }
    }

    /// @name Empty
    /// Constructs an empty string.
    ///
    /// Allocates no buffer; the empty string is represented by a null
    /// pointer with zero length and capacity. Required by
    /// `Defaultable`. The buffer, once there is one, comes from a
    /// default-made `A`.
    ///
    /// # Examples
    ///
    /// ```
    /// let s = String();
    /// s.isEmpty;     // true
    /// s.bytes.count; // 0
    /// ```
    public init() {
        self.init(allocator: A())
    }

    /// @name With Capacity
    /// Constructs an empty string with at least `capacity` bytes preallocated.
    ///
    /// Useful before a series of appends whose total byte count is
    /// known: avoids the geometric-growth reallocations the default
    /// constructor would incur. A non-positive `capacity` is treated
    /// as zero.
    ///
    /// # Errors
    ///
    /// Panics with `"String allocation failed"` if the allocator
    /// returns null.
    ///
    /// # Examples
    ///
    /// ```
    /// var s = String(capacity: 64);
    /// s.bytes.count;  // 0
    /// s.capacity;   // 64
    /// ```
    public init(capacity capacity: Int64) {
        self.init(capacity: capacity, allocator: A())
    }

    /// @name String Literal
    /// Compiler-emitted constructor for string literals.
    ///
    /// Receives a pointer into the literal pool and a length, and borrows
    /// those bytes without copying them: the storage records `cap == 0`
    /// ("owns nothing"). The first write copies the bytes into a heap
    /// buffer (`grow`), so the literal pool is never written or freed.
    ///
    /// `ptr` must stay valid for the whole program, as literal-pool
    /// bytes do.
    public init(stringLiteral ptr: lang.ptr[lang.i8], length: lang.i64) {
        let byteLen = Int64(intLiteral: length);
        let bytes = if byteLen > 0 {
            RawPointer(raw: ptr).cast[UInt8]()
        } else {
            Pointer[UInt8].nullPointer()
        };
        self.storage = CowBox(StringStorage(ptr: bytes, len: byteLen, cap: 0, allocator: A()));
    }

    /// @name From UTF-8
    /// Constructs a string from validated UTF-8 bytes, returning `null`
    /// if the input is not valid UTF-8.
    ///
    /// # Examples
    ///
    /// ```
    /// let s = String(fromUtf8: "héllo".bytes);  // Some("héllo")
    /// ```
    public init(fromUtf8 fromUtf8: &Slice[UInt8])? {
        let bytes = fromUtf8.asUnsafeSlice();
        let count = bytes.count;
        if count == 0 {
            self.storage = CowBox(StringStorage(
                ptr: Pointer[UInt8].nullPointer(),
                len: 0,
                cap: 0,
                allocator: A()
            ))
        } else {
            let rawPtr: lang.ptr[lang.i8] = lang.cast_ptr[_, lang.i8](bytes.pointer.asRaw().raw);
            var i: Int64 = 0;
            while i < count {
                match decodeUtf8(rawPtr, count, at: i) {
                    .Some(decoded) => i = i + decoded.bytesConsumed,
                    .None => return null
                }
            }
            self.init(copying: bytes.pointer, count: count, allocator: A())
        }
    }

    /// @name From UTF-8 Unchecked
    /// Constructs a string by copying bytes without UTF-8 validation.
    ///
    /// # Safety
    ///
    /// The caller must ensure the bytes are valid UTF-8.
    public init(fromUtf8Unchecked fromUtf8Unchecked: &Slice[UInt8]) {
        let bytes = fromUtf8Unchecked.asUnsafeSlice();
        let count = bytes.count;
        if count == 0 {
            self.storage = CowBox(StringStorage(
                ptr: Pointer[UInt8].nullPointer(),
                len: 0,
                cap: 0,
                allocator: A()
            ))
        } else {
            self.init(copying: bytes.pointer, count: count, allocator: A())
        }
    }

    /// @name From UTF-8 Lossy
    /// Constructs a string from bytes, replacing invalid UTF-8 sequences
    /// with the Unicode replacement character (U+FFFD).
    ///
    /// # Examples
    ///
    /// ```
    /// let s = String(fromUtf8Lossy: mixedBytes);  // invalid bytes become '�'
    /// ```
    public init(fromUtf8Lossy fromUtf8Lossy: &Slice[UInt8]) {
        let bytes = fromUtf8Lossy.asUnsafeSlice();
        let count = bytes.count;
        if count == 0 {
            self.storage = CowBox(StringStorage(
                ptr: Pointer[UInt8].nullPointer(),
                len: 0,
                cap: 0,
                allocator: A()
            ))
        } else {
            // Worst case: every byte is invalid → 3 bytes per replacement char
            var result = StringBuilder[A](capacity: count * 3);
            let rawPtr: lang.ptr[lang.i8] = lang.cast_ptr[_, lang.i8](bytes.pointer.asRaw().raw);
            var i: Int64 = 0;
            while i < count {
                match decodeUtf8(rawPtr, count, at: i) {
                    .Some(decoded) => {
                        result.append(char: decoded.char);
                        i = i + decoded.bytesConsumed
                    },
                    .None => {
                        result.append(char: Char(unchecked: UInt32(intLiteral: 0xFFFD)));
                        i = i + 1
                    }
                }
            }
            let built = result.build();
            self.storage = built.storage
        }
    }
}

// Byte-copying helpers for the rest of std.text, which builds default
// strings.
extend String[GlobalAllocator] {
    /// Internal helper: copies `count` bytes from `ptr` without validation.
    static func fromBytesUnchecked(ptr: Pointer[UInt8], count: Int64) -> String {
        String(copying: ptr, count: count, allocator: GlobalAllocator())
    }

    /// Internal helper: copies `count` bytes from a raw `lang.ptr[lang.i8]`.
    static func fromRawBytes(rawPtr: lang.ptr[lang.i8], count: Int64) -> String {
        if count <= 0 {
            return String()
        }
        let typedPtr: lang.ptr[UInt8] = lang.cast_ptr[_, UInt8](rawPtr);
        String.fromBytesUnchecked(Pointer(raw: typedPtr), count)
    }
}

/// `String` lends its text as a `StringSlice`: every read-only text method
/// (`split`, `trimmed`, `contains`, …) is reached through this peel, and a
/// `String` passes wherever a `&StringSlice` is expected.
extend String[A]: Indirection {
    type Target = StringSlice
    public var value: StringSlice {
        ref { self.asSlice() }
    }
}
