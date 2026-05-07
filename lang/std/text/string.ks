// String - UTF-8 encoded string with COW (Copy-on-Write) semantics

module std.text

import std.core.(Bool, Equatable, Matchable, Comparable, Cloneable, Ordering, Addable, ExpressibleByStringLiteral, Hashable, Hasher, Defaultable, fatalError)
import std.text.(Formattable, StringBuilder, _writePadded)
import std.numeric.(Int64, UInt8)
import std.result.(Optional)
import std.memory.(Layout, Pointer, RawPointer, SystemAllocator, RcBox, CowBox, ArraySlice)
import std.iter.(Iterator, Iterable)
import std.collections.(Slice)
import std.numeric.(UInt32)
import std.text.(Char, decodeUtf8, encodeUtf8, BytesView, CharsView, CharsIterator, GraphemesView, LinesView, CharsSubstringIndex, StringSlice, Str, SplitView, SplitWhereView)
import std.text.unicode as unicode
import std.ffi.(memcpy, memcmp, memmem)

// ============================================================================
// INTERNAL HELPERS
// ============================================================================

/// Bulk byte copy via libc `memcpy`. Caller ensures the regions are
/// disjoint and each is valid for `n` bytes; `n <= 0` is a no-op.
func _memcpyBytes(dst dst: Pointer[UInt8], src src: Pointer[UInt8], n n: Int64) {
    if n <= 0 {
        return
    }
    let _ = memcpy(dst.asRaw(), src.asRaw(), n);
}

/// Byte-wise equality of two regions via libc `memcmp`. Caller ensures
/// each region is valid for `n` bytes. `n <= 0` reports equal.
func _bytesEqual(a a: Pointer[UInt8], b b: Pointer[UInt8], n n: Int64) -> Bool {
    if n <= 0 {
        return true
    }
    memcmp(a.asRaw(), b.asRaw(), n) == 0
}

/// Allocates a typed `Pointer[UInt8]` of `layout` bytes, panicking on
/// allocator failure. Centralizes the `SystemAllocator()`/cast/panic
/// boilerplate that every constructor would otherwise duplicate.
func _textAlloc(layout: Layout) -> Pointer[UInt8] {
    var allocator = SystemAllocator();
    match allocator.allocate(layout) {
        .Some(p) => p.cast[UInt8](),
        .None => fatalError("String allocation failed")
    }
}

/// Frees a buffer previously returned by `_textAlloc`.
func _textDealloc(ptr: Pointer[UInt8], layout: Layout) {
    var allocator = SystemAllocator();
    allocator.deallocate(ptr.asRaw(), layout)
}

/// Lexicographic byte-wise comparison via libc `memcmp`. Returns the
/// `Ordering` for the first `n` bytes of the two regions.
func _bytesCompare(a a: Pointer[UInt8], b b: Pointer[UInt8], n n: Int64) -> Ordering {
    if n <= 0 {
        return .Equal
    }
    let r = memcmp(a.asRaw(), b.asRaw(), n);
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

/// Internal heap buffer for `String` — the value sitting inside the `RcBox`.
///
/// Owns a `Pointer[UInt8]` plus its `len` and `cap`. Cloning allocates
/// a fresh exact-fit buffer (used by COW); the deinit deallocates if
/// `cap > 0`. Empty strings carry a null pointer with `len == cap == 0`
/// so the deinit is a no-op.
///
/// # Representation
///
/// `(ptr: Pointer[UInt8], len: Int64, cap: Int64)`.
struct StringStorage: Cloneable {
    var ptr: Pointer[UInt8]
    var len: Int64
    var cap: Int64

    /// @name From Fields
    /// Constructs a storage record from a pointer, length, and capacity.
    ///
    /// Internal: callers must ensure `ptr` is either a fresh allocation
    /// of `cap` bytes (with `len <= cap`) or a null pointer with both
    /// counts zero.
    init(ptr ptr: Pointer[UInt8], len len: Int64, cap cap: Int64) {
        self.ptr = ptr;
        self.len = len;
        self.cap = cap;
    }

    /// Allocates a new exact-fit buffer and copies the bytes.
    ///
    /// Used when COW detects shared storage and a mutation is about to
    /// happen. The clone has `cap == len` regardless of the source's
    /// capacity to avoid carrying slack into copies.
    func clone() -> StringStorage {
        if self.len == 0 {
            return StringStorage(
                ptr: Pointer[UInt8].nullPointer(),
                len: 0,
                cap: 0
            )
        }
        let layout = Layout.array[UInt8](self.len);
        let newPtr = _textAlloc(layout);
        _memcpyBytes(dst: newPtr, src: self.ptr, n: self.len);
        StringStorage(ptr: newPtr, len: self.len, cap: self.len)
    }

    /// Frees the buffer if any was allocated.
    deinit {
        if self.cap > 0 {
            _textDealloc(self.ptr, Layout.array[UInt8](self.cap))
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
/// s.byteCount;            // 12
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
/// - `byteCount`, `capacity`, and `isEmpty` are O(1); `count` (code
///   points) is O(n).
/// - Clones do not share mutation; `s.clone()` and `s` will diverge as
///   soon as either is mutated.
public struct String: Str, Iterable, Equatable, Matchable, Comparable, Cloneable, Formattable, Addable, ExpressibleByStringLiteral, Hashable, Defaultable {
    /// The element type yielded by iteration — always `Char`.
    type Item = Char
    /// The iterator type returned by `iter()`.
    type TargetIterator = CharsIterator
    /// The output type of `+` (concatenation) — always `String`.
    type Output = String

    /// The additive identity for strings — the empty string `""`.
    public static var zero: String { get { "" } }

    private var storage: CowBox[StringStorage]

    // Helper accessors for storage fields
    private func ptr() -> Pointer[UInt8] { self.storage.read().ptr }
    private func len() -> Int64 { self.storage.read().len }
    private func cap() -> Int64 { self.storage.read().cap }

    // ========================================================================
    // CONSTRUCTORS
    // ========================================================================

    /// @name Empty
    /// Constructs an empty string.
    ///
    /// Allocates no buffer; the empty string is represented by a null
    /// pointer with zero length and capacity. Required by
    /// `Defaultable`.
    ///
    /// # Examples
    ///
    /// ```
    /// let s = String();
    /// s.isEmpty;     // true
    /// s.byteCount;   // 0
    /// ```
    public init() {
        self.storage = CowBox(StringStorage(
            ptr: Pointer[UInt8].nullPointer(),
            len: 0,
            cap: 0
        ));
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
    /// Panics with `"String allocation failed"` if the system
    /// allocator returns null.
    ///
    /// # Examples
    ///
    /// ```
    /// var s = String(capacity: 64);
    /// s.byteCount;  // 0
    /// s.capacity;   // 64
    /// ```
    public init(capacity capacity: Int64) {
        if capacity > 0 {
            self.storage = CowBox(StringStorage(
                ptr: _textAlloc(Layout.array[UInt8](capacity)),
                len: 0,
                cap: capacity
            ))
        } else {
            self.storage = CowBox(StringStorage(
                ptr: Pointer[UInt8].nullPointer(),
                len: 0,
                cap: 0
            ))
        }
    }

    /// @name String Literal
    /// Compiler-emitted constructor for string literals.
    ///
    /// Receives a static byte pointer and length, then memcpys into a
    /// fresh heap allocation so the resulting `String` owns its bytes
    /// (and can be mutated independently of the literal pool).
    ///
    /// # Errors
    ///
    /// Panics with `"String allocation failed"` if the system
    /// allocator returns null.
    public init(stringLiteral ptr: lang.ptr[lang.i8], length: lang.i64) {
        let byteCount = Int64(intLiteral: length);
        if byteCount > 0 {
            let newPtr = _textAlloc(Layout.array[UInt8](byteCount));
            let _ = memcpy(newPtr.asRaw(), RawPointer(raw: ptr), byteCount);
            self.storage = CowBox(StringStorage(
                ptr: newPtr,
                len: byteCount,
                cap: byteCount
            ))
        } else {
            self.storage = CowBox(StringStorage(
                ptr: Pointer[UInt8].nullPointer(),
                len: 0,
                cap: 0
            ))
        }
    }

    /// @name From Storage
    /// Wraps an existing `CowBox[StringStorage]` as a new `String`.
    ///
    /// Module-internal — used by `clone()`, `StringBuilder.build()`,
    /// and other std.text code that constructs strings from raw storage.
    init(storage storage: CowBox[StringStorage]) {
        self.storage = storage;
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
    public init[S](fromUtf8 fromUtf8: S)? where S: Slice[UInt8] {
        let bytes = fromUtf8.asSlice();
        let count = bytes.count;
        if count == 0 {
            self.storage = CowBox(StringStorage(
                ptr: Pointer[UInt8].nullPointer(),
                len: 0,
                cap: 0
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
            let newPtr = _textAlloc(Layout.array[UInt8](count));
            _memcpyBytes(dst: newPtr, src: bytes.pointer, n: count);
            self.storage = CowBox(StringStorage(ptr: newPtr, len: count, cap: count))
        }
    }

    /// @name From UTF-8 Unchecked
    /// Constructs a string by copying bytes without UTF-8 validation.
    ///
    /// # Safety
    ///
    /// The caller must ensure the bytes are valid UTF-8.
    public init[S](fromUtf8Unchecked fromUtf8Unchecked: S) where S: Slice[UInt8] {
        let bytes = fromUtf8Unchecked.asSlice();
        let count = bytes.count;
        if count == 0 {
            self.storage = CowBox(StringStorage(
                ptr: Pointer[UInt8].nullPointer(),
                len: 0,
                cap: 0
            ))
        } else {
            let newPtr = _textAlloc(Layout.array[UInt8](count));
            _memcpyBytes(dst: newPtr, src: bytes.pointer, n: count);
            self.storage = CowBox(StringStorage(ptr: newPtr, len: count, cap: count))
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
    public init[S](fromUtf8Lossy fromUtf8Lossy: S) where S: Slice[UInt8] {
        let bytes = fromUtf8Lossy.asSlice();
        let count = bytes.count;
        if count == 0 {
            self.storage = CowBox(StringStorage(
                ptr: Pointer[UInt8].nullPointer(),
                len: 0,
                cap: 0
            ))
        } else {
            // Worst case: every byte is invalid → 3 bytes per replacement char
            var result = StringBuilder(capacity: count * 3);
            let rawPtr: lang.ptr[lang.i8] = lang.cast_ptr[_, lang.i8](bytes.pointer.asRaw().raw);
            var i: Int64 = 0;
            while i < count {
                match decodeUtf8(rawPtr, count, at: i) {
                    .Some(decoded) => {
                        result.appendChar(decoded.char);
                        i = i + decoded.bytesConsumed
                    },
                    .None => {
                        result.appendChar(Char(unchecked: UInt32(intLiteral: 0xFFFD)));
                        i = i + 1
                    }
                }
            }
            let built = result.build();
            self.storage = built.storage
        }
    }

    /// Internal helper: copies `count` bytes from `ptr` without validation.
    static func fromBytesUnchecked(ptr: Pointer[UInt8], count: Int64) -> String {
        if count == 0 {
            return String()
        }
        let newPtr = _textAlloc(Layout.array[UInt8](count));
        _memcpyBytes(dst: newPtr, src: ptr, n: count);
        String(storage: CowBox(StringStorage(ptr: newPtr, len: count, cap: count)))
    }

    /// Internal helper: copies `count` bytes from a raw `lang.ptr[lang.i8]`.
    static func fromRawBytes(rawPtr: lang.ptr[lang.i8], count: Int64) -> String {
        if count <= 0 {
            return String()
        }
        let typedPtr: lang.ptr[UInt8] = lang.cast_ptr[_, UInt8](rawPtr);
        String.fromBytesUnchecked(Pointer(raw: typedPtr), count)
    }

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
    // STR CONFORMANCE
    // ========================================================================

    /// Returns a `StringSlice` covering this string's entire buffer.
    /// Shares storage via refcount — zero-copy.
    public func asSlice() -> StringSlice {
        StringSlice(source: self.storage.shareBox(), start: 0, end: self.len())
    }

    // ========================================================================
    // SIZE & CAPACITY
    // ========================================================================

    /// The number of UTF-8 bytes in the string. O(1).
    ///
    /// This is **not** the character count — see `count` for that.
    /// Pure ASCII strings have `byteCount == count`.
    public var byteCount: Int64 { self.len() }

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
        let newPtr = _textAlloc(newLayout);
        let oldStorage = self.storage.write();
        _memcpyBytes(dst: newPtr, src: oldStorage.ptr, n: oldStorage.len);
        self.storage.setValue(StringStorage(ptr: newPtr, len: oldStorage.len, cap: newCap))
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
    public mutating func append(other: String) {
        let otherLen = other.len();
        if otherLen == 0 {
            return
        }
        let myLen = self.len();
        self.grow(myLen + otherLen);
        var s = self.storage.write();
        _memcpyBytes(dst: s.ptr.offset(by: s.len), src: other.ptr(), n: otherLen);
        s.len = s.len + otherLen;
        self.storage.setValue(s)
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
    /// s.appendChar('i');
    /// s.appendChar('\u{1F600}');
    /// s;  // "hi😀"
    /// ```
    public mutating func appendChar(c: Char) {
        let utf8Len = c.utf8Length();
        self.grow(self.len() + utf8Len);
        var s = self.storage.write();
        // Encode to buffer
        let rawPtr: lang.ptr[lang.i8] = lang.cast_ptr[_, lang.i8](s.ptr.asRaw().raw);
        let written = encodeUtf8(c, rawPtr, at: s.len);
        s.len = s.len + written;
        self.storage.setValue(s)
    }

    /// Appends a raw byte. Internal — caller ensures UTF-8 validity.
    ///
    /// Do not use to append ASCII characters: prefer `appendChar(c)` or
    /// `append(other)`. This exists only for low-level UTF-8 plumbing
    /// inside the stdlib (e.g. an encoder that already produced bytes).
    internal mutating func appendByte(byte: UInt8) {
        self.grow(self.len() + 1);
        var s = self.storage.write();
        s.ptr.offset(by: s.len).write(byte);
        s.len = s.len + 1;
        self.storage.setValue(s)
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
        var s = self.storage.write();
        _memcpyBytes(dst: s.ptr.offset(by: s.len), src: ptr, n: n);
        s.len = s.len + n;
        self.storage.setValue(s)
    }

    /// Internal substring by byte range. Returns empty for invalid ranges.
    ///
    /// Do not use for per-character slicing in a loop — each call copies
    /// `end - start` bytes, so walking the string yields O(N²) behaviour.
    /// For iteration, use `decodeUtf8` with a running byte offset, or the
    /// `chars()` / `bytes()` views.
    internal func substringBytes(from start: Int64, to end: Int64) -> String {
        let myLen = self.len();
        if start >= end or start < 0 or end > myLen {
            return String()
        }
        String.fromBytesUnchecked(self.ptr().offset(by: start), end - start)
    }

    /// Truncates the string to length zero, keeping the allocated buffer.
    ///
    /// Capacity is unchanged, so this is the right primitive for
    /// reusing a buffer in a hot loop.
    public mutating func clear() {
        var s = self.storage.write();
        s.len = 0;
        self.storage.setValue(s)
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
        self.trimStart();
        self.trimEnd()
    }

    /// Removes leading ASCII whitespace in place.
    public mutating func trimStart() {
        self = self.trimmedStart().toOwned()
    }

    /// Removes trailing ASCII whitespace in place.
    public mutating func trimEnd() {
        self = self.trimmedEnd().toOwned()
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
        self = self.trimmed(where: predicate).toOwned()
    }

    /// Removes leading code points matching `predicate`, in place.
    public mutating func trimStart(where predicate: (Char) -> Bool) {
        self = self.trimmedStart(where: predicate).toOwned()
    }

    /// Removes trailing code points matching `predicate`, in place.
    public mutating func trimEnd(where predicate: (Char) -> Bool) {
        self = self.trimmedEnd(where: predicate).toOwned()
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
        var s = self.storage.write();
        for i in 0..<myLen {
            let byte = s.ptr.offset(by: i).read();
            let v: lang.i32 = lang.cast_i8_i32(byte.raw);
            // A-Z: 65-90 -> a-z: 97-122
            let isUppercase = lang.i1_and(lang.i32_signed_ge(v, 65), lang.i32_signed_le(v, 90));
            if Bool(boolLiteral: isUppercase) {
                s.ptr.offset(by: i).write(UInt8(raw: lang.cast_i32_i8(lang.i32_add(v, 32))))
            }
        }
        self.storage.setValue(s)
    }

    /// Uppercases ASCII letters in place; non-ASCII bytes are left untouched.
    public mutating func uppercaseAscii() {
        let myLen = self.len();
        var s = self.storage.write();
        for i in 0..<myLen {
            let byte = s.ptr.offset(by: i).read();
            let v: lang.i32 = lang.cast_i8_i32(byte.raw);
            // a-z: 97-122 -> A-Z: 65-90
            let isLowercase = lang.i1_and(lang.i32_signed_ge(v, 97), lang.i32_signed_le(v, 122));
            if Bool(boolLiteral: isLowercase) {
                s.ptr.offset(by: i).write(UInt8(raw: lang.cast_i32_i8(lang.i32_sub(v, 32))))
            }
        }
        self.storage.setValue(s)
    }

    // ========================================================================
    // CASE CONVERSION (Unicode, mutating)
    // ========================================================================

    /// Replaces this string with its lowercase form using full Unicode case mapping.
    public mutating func lowercase() {
        self = self.lowercased()
    }

    /// Replaces this string with its uppercase form using full Unicode case mapping.
    ///
    /// Locale-independent. Handles multi-character expansions —
    /// e.g. German `ß` → `SS`.
    public mutating func uppercase() {
        self = self.uppercased()
    }

    // ========================================================================
    // REPLACEMENT (Mutating)
    // ========================================================================

    /// Replaces every occurrence of `pattern` with `replacement`, in place.
    public mutating func replace(pattern: String, with replacement: String) {
        self = self.replaced(pattern, with: replacement)
    }

    // ========================================================================
    // ITERATION
    // ========================================================================

    /// Returns a `CharsIterator` over the code points starting at byte 0.
    ///
    /// Required by `Iterable`. Each call returns a fresh iterator;
    /// the string itself is reusable.
    public func iter() -> CharsIterator {
        CharsIterator(ptr: lang.cast_ptr[_, lang.i8](self.ptr().asRaw().raw), length: self.len(), byteIndex: 0)
    }

    // ========================================================================
    // PROTOCOL CONFORMANCES
    // ========================================================================

    /// Returns the concatenation `self + other`. Required by `Addable`.
    ///
    /// Equivalent to cloning `self` and appending `other`.
    public func add(other: String) -> String {
        var result = self.clone();
        result.append(other);
        result
    }

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
    public func isEqual(to other: String) -> Bool {
        let myLen = self.len();
        let otherLen = other.len();
        if myLen != otherLen {
            return false
        }
        _bytesEqual(a: self.ptr(), b: other.ptr(), n: myLen)
    }

    /// Pattern-match form of `isEqual`: each `case "literal" =>` arm
    /// dispatches through here. Cost is `O(len)` per arm because the
    /// compiler emits one call per literal — past a handful of arms,
    /// E316 will suggest an `if/else if` chain instead.
    public func matches(other: String) -> Bool {
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
    public func compare(other: String) -> Ordering {
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
        hasher.write(ArraySlice(pointer: self.ptr(), count: self.len()))
    }

    /// Returns a shallow clone — storage is shared until either side mutates.
    ///
    /// O(1). Mutation triggers a deep copy via `CowBox.write()`.
    public func clone() -> String {
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
    /// "test".format(opts);   // "test      "
    /// opts.alignment = .Right;
    /// "test".format(opts);   // "      test"
    /// opts.alignment = .Center;
    /// "test".format(opts);   // "   test   "
    /// ```
    public func format(mutating into writer: StringBuilder, options: FormatOptions = FormatOptions.default()) {
        _writePadded(into: writer, self, options)
    }
}

extend String {
    /// @name From Char Iterable
    /// Builds a string by encoding each character of `chars` as UTF-8.
    ///
    /// Mirrors `Array.init(from:)` and `Set.init(from:)` — accepts any
    /// `Iterable` whose `Item` is `Char`. Useful for materializing the
    /// result of an iterator chain back into a `String`:
    ///
    /// ```
    /// let upper = String(from: "hello".chars.iter().map { it.toUpper() });
    /// // "HELLO"
    /// ```
    public init[I](from chars: I) where I: Iterable, I.Item = Char {
        var b = StringBuilder();
        var iter = chars.iter();
        while let .Some(c) = iter.next() {
            b.appendChar(c)
        }
        self = b.build();
    }
}
