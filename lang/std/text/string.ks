// String - UTF-8 encoded string with COW (Copy-on-Write) semantics

module std.text

import std.core.(Bool, Equatable, Comparable, Cloneable, Ordering, Addable, ExpressibleByStringLiteral, Hash, Hasher, Defaultable)
import std.text.(Formattable)
import std.num.(Int64, UInt8)
import std.result.(Optional)
import std.memory.(Layout, Pointer, RawPointer, SystemAllocator, RcBox, Slice)
import std.iter.(Iterator, Iterable)
import std.text.(Char, decodeUtf8, encodeUtf8, BytesView, CharsView, GraphemesView, LinesView)
import std.text.unicode as unicode
import std.ffi.(memcpy)

// ============================================================================
// STRING ITERATOR
// ============================================================================

/// Single-pass forward iterator over the Unicode code points of a `String`.
///
/// Produced by `String.iter()`. Decodes one UTF-8 character at a time,
/// advancing the cursor by the encoded byte length. On invalid UTF-8
/// the iterator returns `None` and skips one byte so the next call
/// can make progress; this differs from `CharsIterator` which yields
/// `U+FFFD` on bad input.
///
/// # Examples
///
/// ```
/// var it = "hi".iter();
/// it.next();  // Some('h')
/// it.next();  // Some('i')
/// it.next();  // None
/// ```
///
/// # Representation
///
/// A `(ptr, length, index)` triple. `index` advances in variable-width
/// steps according to the UTF-8 encoding.
///
/// # Memory Model
///
/// Value type. The pointer aliases the source string's storage; do not
/// retain across mutations of the source `String`.
public struct StringIterator: Iterator {
    /// The element type yielded by `next()` — always `Char`.
    type Item = Char

    private var ptr: Pointer[UInt8]
    private var length: Int64
    private var index: Int64

    /// @name From Pointer
    /// Constructs a string iterator from a buffer pointer and total byte count.
    ///
    /// Prefer `someString.iter()` over calling this directly.
    ///
    /// # Safety
    ///
    /// `ptr` must point to `length` valid UTF-8 bytes that remain live
    /// for the iterator's lifetime.
    public init(ptr ptr: Pointer[UInt8], length length: Int64) {
        self.ptr = ptr;
        self.length = length;
        self.index = Int64(intLiteral: 0);
    }

    /// Returns the next code point, or `None` when the buffer is exhausted.
    ///
    /// On invalid UTF-8 the iterator returns `None` and advances by one
    /// byte to guarantee forward progress on subsequent calls.
    public mutating func next() -> Char? {
        if self.index >= self.length {
            return .None
        }
        // Decode UTF-8 at current position
        let rawPtr: lang.ptr[lang.i8] = lang.cast_ptr[_, lang.i8](self.ptr.asRaw().raw);
        let result = decodeUtf8(rawPtr, self.length, at: self.index);
        if let .Some(decoded) = result {
            self.index = self.index + decoded.bytesConsumed;
            .Some(decoded.char)
        } else {
            // Invalid UTF-8, skip one byte
            self.index = self.index + Int64(intLiteral: 1);
            .None
        }
    }
}

// ============================================================================
// SPLIT ITERATOR
// ============================================================================

/// Iterator that yields the segments produced by splitting a string on a fixed-byte separator.
///
/// Produced by `String.split(separator:)`. Walks the source byte-by-byte
/// looking for an exact match of the separator's bytes (no UTF-8
/// awareness needed — the separator itself is UTF-8 so its byte
/// pattern can never align inside a multi-byte sequence). The empty
/// separator is treated specially: it splits per code point.
///
/// # Examples
///
/// ```
/// var it = "a,b,c".split(separator: ",");
/// it.next();  // Some("a")
/// it.next();  // Some("b")
/// it.next();  // Some("c")
/// it.next();  // None
/// ```
///
/// # Representation
///
/// A `(ptr, length, sepPtr, sepLen, index, done)` record. `done` flips
/// once the trailing remainder has been emitted.
///
/// # Memory Model
///
/// Value type. Borrows both the source and the separator buffers; do
/// not retain across mutations of either.
public struct SplitIterator: Iterator {
    /// The element type yielded by `next()` — always `String`.
    type Item = String

    private var ptr: Pointer[UInt8]
    private var length: Int64
    private var sepPtr: Pointer[UInt8]
    private var sepLen: Int64
    private var index: Int64
    private var done: Bool

    /// @name From Pointers
    /// Constructs a split iterator from source and separator byte buffers.
    ///
    /// Prefer `someString.split(separator:)` over calling this directly.
    ///
    /// # Safety
    ///
    /// Both pointers must remain valid for `length` and `sepLen` bytes
    /// respectively for the iterator's lifetime.
    public init(ptr ptr: Pointer[UInt8], length length: Int64, sepPtr sepPtr: Pointer[UInt8], sepLen sepLen: Int64) {
        self.ptr = ptr;
        self.length = length;
        self.sepPtr = sepPtr;
        self.sepLen = sepLen;
        self.index = Int64(intLiteral: 0);
        self.done = false;
    }

    /// Returns the next segment, or `None` when the source is exhausted.
    ///
    /// With a non-empty separator, returns each piece between matches
    /// and finally the trailing remainder. With the empty separator,
    /// returns one code point per call.
    public mutating func next() -> String? {
        if self.done {
            return .None
        }

        let start = self.index;

        if self.sepLen == Int64(intLiteral: 0) {
            // Empty separator - split by code point
            if self.index >= self.length {
                self.done = true;
                return .None
            }
            // Decode one code point
            let rawPtr: lang.ptr[lang.i8] = lang.cast_ptr[_, lang.i8](self.ptr.asRaw().raw);
            let result = decodeUtf8(rawPtr, self.length, at: self.index);
            if let .Some(decoded) = result {
                self.index = self.index + decoded.bytesConsumed;
                return .Some(String.fromBytesUnchecked(self.ptr.offset(by: start), decoded.bytesConsumed))
            }
            self.done = true;
            return .None
        }

        // Search for separator
        var found: Bool = false;
        var matchIndex: Int64 = self.index;
        while self.index + self.sepLen <= self.length and found == false {
            var matches: Bool = true;
            var j: Int64 = Int64(intLiteral: 0);
            while j < self.sepLen and matches {
                let a = self.ptr.offset(by: self.index + j).read();
                let b = self.sepPtr.offset(by: j).read();
                if a.equals(b) == false {
                    matches = false
                }
                j = j + Int64(intLiteral: 1)
            }
            if matches {
                found = true;
                matchIndex = self.index;
                self.index = self.index + self.sepLen
            } else {
                self.index = self.index + Int64(intLiteral: 1)
            }
        }

        if found {
            return .Some(String.fromBytesUnchecked(self.ptr.offset(by: start), matchIndex - start))
        }

        // No more separators - return remainder
        if start < self.length {
            self.done = true;
            return .Some(String.fromBytesUnchecked(self.ptr.offset(by: start), self.length - start))
        }

        self.done = true;
        .None
    }
}

// ============================================================================
// SPLIT WHERE ITERATOR
// ============================================================================

/// Iterator that splits a string at every code point matching a predicate.
///
/// Produced by `String.split(matching:)`. Decodes the source one
/// `Char` at a time and breaks the string at each character for which
/// the predicate returns `true`; the matching character itself is not
/// included in any segment.
///
/// # Examples
///
/// ```
/// var it = "a1b2c".split(matching: |c| c.isDigit());
/// it.next();  // Some("a")
/// it.next();  // Some("b")
/// it.next();  // Some("c")
/// it.next();  // None
/// ```
///
/// # Representation
///
/// A `(ptr, length, predicate, index, done)` record.
public struct SplitWhereIterator: Iterator {
    /// The element type yielded by `next()` — always `String`.
    type Item = String

    private var ptr: Pointer[UInt8]
    private var length: Int64
    private var predicate: (Char) -> Bool
    private var index: Int64
    private var done: Bool

    /// @name From Predicate
    /// Constructs a split-where iterator from a buffer pointer and a `Char` predicate.
    ///
    /// Prefer `someString.split(matching:)` over calling this directly.
    ///
    /// # Safety
    ///
    /// `ptr` must remain valid for `length` bytes for the iterator's
    /// lifetime.
    public init(ptr ptr: Pointer[UInt8], length length: Int64, predicate predicate: (Char) -> Bool) {
        self.ptr = ptr;
        self.length = length;
        self.predicate = predicate;
        self.index = Int64(intLiteral: 0);
        self.done = false;
    }

    /// Returns the next segment, or `None` when the source is exhausted.
    public mutating func next() -> String? {
        if self.done {
            return .None
        }

        let start = self.index;

        // Search for character matching predicate
        var found: Bool = false;
        var matchIndex: Int64 = self.index;
        while self.index < self.length and found == false {
            // Decode UTF-8 at current position
            let rawPtr: lang.ptr[lang.i8] = lang.cast_ptr[_, lang.i8](self.ptr.asRaw().raw);
            let result = decodeUtf8(rawPtr, self.length, at: self.index);
            if let .Some(decoded) = result {
                if self.predicate(decoded.char) {
                    found = true;
                    matchIndex = self.index;
                    self.index = self.index + decoded.bytesConsumed
                } else {
                    self.index = self.index + decoded.bytesConsumed
                }
            } else {
                self.index = self.index + Int64(intLiteral: 1)
            }
        }

        if found {
            return .Some(String.fromBytesUnchecked(self.ptr.offset(by: start), matchIndex - start))
        }

        // No more matches - return remainder
        if start < self.length {
            self.done = true;
            return .Some(String.fromBytesUnchecked(self.ptr.offset(by: start), self.length - start))
        }

        self.done = true;
        .None
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
        if self.len == Int64(intLiteral: 0) {
            return StringStorage(
                ptr: Pointer(raw: lang.ptr_null[UInt8]()),
                len: Int64(intLiteral: 0),
                cap: Int64(intLiteral: 0)
            )
        }
        let layout = Layout.array[UInt8](self.len);
        var allocator = SystemAllocator();
        let result = allocator.allocate(layout);
        if let .Some(allocated) = result {
            let newPtr = allocated.cast[UInt8]();
            // Copy bytes
            for i in Int64(intLiteral: 0)..<self.len {
                newPtr.offset(by: i).write(self.ptr.offset(by: i).read())
            }
            StringStorage(ptr: newPtr, len: self.len, cap: self.len)
        } else {
            lang.panic("StringStorage clone allocation failed")
        }
    }

    /// Frees the buffer if any was allocated.
    deinit {
        if self.cap > Int64(intLiteral: 0) {
            let layout = Layout.array[UInt8](self.cap);
            var allocator = SystemAllocator();
            allocator.deallocate(self.ptr.asRaw(), layout)
        }
    }
}

// ============================================================================
// STRING
// ============================================================================

/// A UTF-8 encoded, dynamically sized string with copy-on-write semantics.
///
/// `String` is the standard text type. The bytes are always valid
/// UTF-8 except after the unsafe internal `appendByte` path, which is
/// only intended for callers (such as substring helpers) that already
/// know the bytes are valid. Storage is shared between clones via an
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
/// s.contains(substring: ",");  // true
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
/// A single `RcBox[StringStorage]` field. The storage record carries
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
public struct String: Iterable, Equatable, Comparable, Cloneable, Formattable, Addable, ExpressibleByStringLiteral, Hash, Defaultable {
    /// The element type yielded by iteration — always `Char`.
    type Item = Char
    /// The iterator type returned by `iter()`.
    type Iter = StringIterator
    /// The output type of `+` (concatenation) — always `String`.
    type Output = String

    /// The additive identity for strings — the empty string `""`.
    public static var zero: String { get { "" } }

    private var storage: RcBox[StringStorage]

    // Helper accessors for storage fields
    private func ptr() -> Pointer[UInt8] { self.storage.getValue().ptr }
    private func len() -> Int64 { self.storage.getValue().len }
    private func cap() -> Int64 { self.storage.getValue().cap }

    // Ensure unique storage for mutation (COW)
    private mutating func makeUnique() {
        if self.storage.isUnique() == false {
            self.storage = RcBox(self.storage.getValue().clone())
        }
    }

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
        self.storage = RcBox(StringStorage(
            ptr: Pointer(raw: lang.ptr_null[UInt8]()),
            len: Int64(intLiteral: 0),
            cap: Int64(intLiteral: 0)
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
        if capacity > Int64(intLiteral: 0) {
            let layout = Layout.array[UInt8](capacity);
            var allocator = SystemAllocator();
            let result = allocator.allocate(layout);
            if let .Some(allocated) = result {
                self.storage = RcBox(StringStorage(
                    ptr: allocated.cast[UInt8](),
                    len: Int64(intLiteral: 0),
                    cap: capacity
                ))
            } else {
                lang.panic("String allocation failed")
            }
        } else {
            self.storage = RcBox(StringStorage(
                ptr: Pointer(raw: lang.ptr_null[UInt8]()),
                len: Int64(intLiteral: 0),
                cap: Int64(intLiteral: 0)
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
        if lang.i64_signed_gt(length, 0) {
            let byteCount = Int64(intLiteral: length);
            let layout = Layout.array[UInt8](byteCount);
            var allocator = SystemAllocator();
            let result = allocator.allocate(layout);
            if let .Some(allocated) = result {
                let newPtr = allocated.cast[UInt8]();
                // Copy bytes from literal
                let srcPtr: lang.ptr[lang.i8] = ptr;
                let dstPtr: lang.ptr[lang.i8] = lang.cast_ptr[_, lang.i8](newPtr.asRaw().raw);
                let _ = memcpy(dstPtr, srcPtr, length);
                self.storage = RcBox(StringStorage(
                    ptr: newPtr,
                    len: byteCount,
                    cap: byteCount
                ))
            } else {
                lang.panic("String allocation failed")
            }
        } else {
            self.storage = RcBox(StringStorage(
                ptr: Pointer(raw: lang.ptr_null[UInt8]()),
                len: Int64(intLiteral: 0),
                cap: Int64(intLiteral: 0)
            ))
        }
    }

    /// @name From Storage
    /// Wraps an existing `RcBox[StringStorage]` as a new `String`.
    ///
    /// Internal — used by `clone()` to share the existing storage box.
    private init(storage storage: RcBox[StringStorage]) {
        self.storage = storage;
    }

    /// @name From UTF-8
    /// Constructs a string by copying validated UTF-8 bytes from `bytes`,
    /// returning `.None` if the slice is not valid UTF-8.
    ///
    /// Walks the slice end-to-end with `decodeUtf8`; any malformed,
    /// truncated, or overlong sequence produces `.None`. The empty slice
    /// is valid and yields the empty string. On success the bytes are
    /// copied into a fresh heap allocation, so the returned `String`
    /// owns its storage independently of `bytes`.
    ///
    /// # Errors
    ///
    /// Panics with `"String allocation failed"` if the system allocator
    /// returns null. Returns `.None` only for invalid UTF-8 — the
    /// allocation case is unrecoverable.
    ///
    /// # Examples
    ///
    /// ```
    /// String.fromUtf8(bytes: "héllo".bytes.asSlice());  // Some("héllo")
    /// String.fromUtf8(bytes: badSlice);                 // None
    /// ```
    public static func fromUtf8(bytes: Slice[UInt8]) -> String? {
        let count = bytes.count;
        if count == Int64(intLiteral: 0) {
            return .Some(String())
        }
        // Validate: walk the buffer with decodeUtf8 until exhausted.
        let rawPtr: lang.ptr[lang.i8] = lang.cast_ptr[_, lang.i8](bytes.pointer.asRaw().raw);
        var i: Int64 = Int64(intLiteral: 0);
        while i < count {
            match decodeUtf8(rawPtr, count, at: i) {
                .Some(decoded) => i = i + decoded.bytesConsumed,
                .None => return .None
            }
        }
        .Some(String.fromBytesUnchecked(bytes.pointer, count))
    }

    /// Constructs a string by copying `count` bytes starting at `ptr`, without UTF-8 validation.
    ///
    /// Internal helper used by split iterators and substring helpers
    /// that already know the byte range falls on UTF-8 boundaries.
    ///
    /// # Safety
    ///
    /// `ptr` must reference at least `count` valid UTF-8 bytes; the
    /// range starting at `ptr` and ending at `ptr + count` must not
    /// split a multi-byte sequence.
    static func fromBytesUnchecked(ptr: Pointer[UInt8], count: Int64) -> String {
        if count == Int64(intLiteral: 0) {
            return String()
        }
        let layout = Layout.array[UInt8](count);
        var allocator = SystemAllocator();
        let result = allocator.allocate(layout);
        if let .Some(allocated) = result {
            let newPtr = allocated.cast[UInt8]();
            // Copy bytes
            for i in Int64(intLiteral: 0)..<count {
                newPtr.offset(by: i).write(ptr.offset(by: i).read())
            }
            String(storage: RcBox(StringStorage(ptr: newPtr, len: count, cap: count)))
        } else {
            lang.panic("String allocation failed")
        }
    }

    // ========================================================================
    // VIEW PROPERTIES
    // ========================================================================

    /// A view over the raw UTF-8 bytes — O(1) byte indexing, byte-level iteration.
    public var bytes: BytesView {
        BytesView(ptr: lang.cast_ptr[_, lang.i8](self.ptr().asRaw().raw), length: self.len())
    }

    /// A view over the Unicode code points — O(n) indexing, scalar-level iteration.
    public var chars: CharsView {
        CharsView(ptr: lang.cast_ptr[_, lang.i8](self.ptr().asRaw().raw), length: self.len())
    }

    /// A view over the user-perceived characters (UAX #29 grapheme clusters).
    public var graphemes: GraphemesView {
        GraphemesView(ptr: lang.cast_ptr[_, lang.i8](self.ptr().asRaw().raw), length: self.len())
    }

    /// A view over the lines of the string, recognising `\n`, `\r\n`, and `\r`.
    public var lines: LinesView {
        LinesView(ptr: lang.cast_ptr[_, lang.i8](self.ptr().asRaw().raw), length: self.len())
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
    public var isEmpty: Bool { self.len() == Int64(intLiteral: 0) }

    /// The number of Unicode code points. O(n).
    ///
    /// Walks the buffer counting UTF-8 leading bytes (those whose top
    /// two bits are not `10`). Cache the result if you need it more
    /// than once.
    ///
    /// # Examples
    ///
    /// ```
    /// "hi".count;     // 2
    /// "héllo".count;  // 5 (code points; bytes is 6)
    /// ```
    public var count: Int64 {
        let myLen = self.len();
        let myPtr = self.ptr();
        var n: Int64 = Int64(intLiteral: 0);
        for i in Int64(intLiteral: 0)..<myLen {
            let byte = myPtr.offset(by: i).read();
            // Count leading bytes only (not continuation bytes 10xxxxxx)
            let byteVal: lang.i32 = lang.cast_i8_i32(byte.raw);
            if lang.i32_ne(lang.i32_and(byteVal, 0xC0), 0x80) {
                n = n + Int64(intLiteral: 1)
            }
        }
        n
    }

    // ========================================================================
    // CHARACTER ACCESS
    // ========================================================================

    /// Returns the first code point, or `None` if the string is empty. O(1) for the common case.
    ///
    /// # Examples
    ///
    /// ```
    /// "hi".first();  // Some('h')
    /// "".first();    // None
    /// ```
    public func first() -> Char? {
        if self.len() == Int64(intLiteral: 0) {
            return .None
        }
        let rawPtr: lang.ptr[lang.i8] = lang.cast_ptr[_, lang.i8](self.ptr().asRaw().raw);
        let result = decodeUtf8(rawPtr, self.len(), at: Int64(intLiteral: 0));
        if let .Some(decoded) = result {
            .Some(decoded.char)
        } else {
            .None
        }
    }

    /// Returns the last code point, or `None` if the string is empty. O(n).
    ///
    /// Has to scan from the start to identify the final UTF-8 sequence
    /// — there is no way to read backwards through variable-width
    /// UTF-8 without a separate index.
    ///
    /// # Examples
    ///
    /// ```
    /// "hi".last();  // Some('i')
    /// "".last();    // None
    /// ```
    public func last() -> Char? {
        let myLen = self.len();
        if myLen == Int64(intLiteral: 0) {
            return .None
        }
        // Scan to find the last character
        let myPtr = self.ptr();
        var lastChar: Char? = .None;
        var i: Int64 = Int64(intLiteral: 0);
        while i < myLen {
            let rawPtr: lang.ptr[lang.i8] = lang.cast_ptr[_, lang.i8](myPtr.asRaw().raw);
            let result = decodeUtf8(rawPtr, myLen, at: i);
            if let .Some(decoded) = result {
                lastChar = .Some(decoded.char);
                i = i + decoded.bytesConsumed
            } else {
                i = i + Int64(intLiteral: 1)
            }
        }
        lastChar
    }

    /// Returns the code point at code-point index `index`. Panics if out of bounds. O(n).
    ///
    /// # Errors
    ///
    /// Panics with `"String index out of bounds"` if `index` is
    /// negative or `>= count`.
    ///
    /// # Examples
    ///
    /// ```
    /// "héllo".char(at: 1);  // 'é'
    /// ```
    public func char(at index: Int64) -> Char {
        match self.char(checked: index) {
            .Some(c) => c,
            .None => lang.panic("String index out of bounds")
        }
    }

    /// Returns the code point at code-point index `index`, or `None` if out of bounds. O(n).
    ///
    /// Companion to `char(at:)`. Walks UTF-8 to find the right
    /// character; the early `return` on the matching index keeps
    /// average-case work proportional to `index`, not to `count`.
    public func char(checked index: Int64) -> Char? {
        let myLen = self.len();
        let myPtr = self.ptr();
        var charIndex: Int64 = Int64(intLiteral: 0);
        var byteIndex: Int64 = Int64(intLiteral: 0);
        while byteIndex < myLen {
            if charIndex == index {
                let rawPtr: lang.ptr[lang.i8] = lang.cast_ptr[_, lang.i8](myPtr.asRaw().raw);
                let result = decodeUtf8(rawPtr, myLen, at: byteIndex);
                if let .Some(decoded) = result {
                    return .Some(decoded.char)
                }
                return .None
            }
            let rawPtr: lang.ptr[lang.i8] = lang.cast_ptr[_, lang.i8](myPtr.asRaw().raw);
            let result = decodeUtf8(rawPtr, myLen, at: byteIndex);
            if let .Some(decoded) = result {
                byteIndex = byteIndex + decoded.bytesConsumed;
                charIndex = charIndex + Int64(intLiteral: 1)
            } else {
                byteIndex = byteIndex + Int64(intLiteral: 1)
            }
        }
        .None
    }

    /// Returns the code point at code-point index `index` without bounds checking.
    ///
    /// Currently delegates to `char(at:)` which itself panics on
    /// out-of-range, so this is not actually faster — exists for API
    /// parity with `Array`'s indexing family.
    ///
    /// # Safety
    ///
    /// Caller must guarantee `0 <= index < count`.
    public func char(unchecked index: Int64) -> Char {
        self.char(at: index)
    }

    /// Returns the code point at `index` modulo `count`, supporting negative indices.
    ///
    /// Negative indices count from the end (`-1` is the last code
    /// point). Equivalent to `char(at: ((index % count) + count) % count)`.
    ///
    /// # Errors
    ///
    /// Panics with `"String is empty"` if the string contains no code
    /// points.
    ///
    /// # Examples
    ///
    /// ```
    /// "abc".char(wrapping: -1);  // 'c'
    /// "abc".char(wrapping: 5);   // 'c'  (5 % 3 == 2)
    /// ```
    public func char(wrapping index: Int64) -> Char {
        let charCount = self.count;
        if charCount == Int64(intLiteral: 0) {
            lang.panic("String is empty")
        }
        var idx = index;
        while idx < Int64(intLiteral: 0) {
            idx = idx + charCount
        }
        idx = idx % charCount;
        self.char(at: idx)
    }

    /// Returns the code point at `index` clamped to `[0, count - 1]`.
    ///
    /// # Errors
    ///
    /// Panics with `"String is empty"` if the string contains no code
    /// points.
    ///
    /// # Examples
    ///
    /// ```
    /// "abc".char(clamping: -1);  // 'a'
    /// "abc".char(clamping: 99);  // 'c'
    /// ```
    public func char(clamping index: Int64) -> Char {
        let charCount = self.count;
        if charCount == Int64(intLiteral: 0) {
            lang.panic("String is empty")
        }
        var idx = index;
        if idx < Int64(intLiteral: 0) {
            idx = Int64(intLiteral: 0)
        }
        if idx >= charCount {
            idx = charCount - Int64(intLiteral: 1)
        }
        self.char(at: idx)
    }

    // ========================================================================
    // BYTE ACCESS
    // ========================================================================

    /// Returns the byte at byte index `index`, or `None` if out of bounds. O(1).
    ///
    /// For unchecked access, use `byteAtUnchecked`. The byte is
    /// returned as raw `UInt8` — multi-byte characters require the
    /// caller to assemble them.
    public func byteAt(index: Int64) -> UInt8? {
        let myLen = self.len();
        if index >= Int64(intLiteral: 0) and index < myLen {
            .Some(self.ptr().offset(by: index).read())
        } else {
            .None
        }
    }

    /// Returns the byte at `index` without bounds checking. O(1).
    ///
    /// # Safety
    ///
    /// Caller must guarantee `0 <= index < byteCount`.
    public func byteAtUnchecked(index: Int64) -> UInt8 {
        self.ptr().offset(by: index).read()
    }

    // ========================================================================
    // CAPACITY MANAGEMENT (Internal)
    // ========================================================================

    /// Grows the buffer to at least `minCapacity` bytes, preserving content.
    ///
    /// Geometric-growth (doubles each step from a base of 16) so an N
    /// append loop stays amortised O(N). Triggers `makeUnique` first
    /// to copy out of any shared storage.
    private mutating func grow(minCapacity: Int64) {
        let myCap = self.cap();
        if myCap >= minCapacity {
            return
        }

        self.makeUnique();

        var newCap: Int64 = myCap;
        if newCap == Int64(intLiteral: 0) {
            newCap = Int64(intLiteral: 16)
        }
        while newCap < minCapacity {
            newCap = newCap * Int64(intLiteral: 2)
        }

        let newLayout = Layout.array[UInt8](newCap);
        var allocator = SystemAllocator();
        let result = allocator.allocate(newLayout);
        if let .Some(allocated) = result {
            let newPtr = allocated.cast[UInt8]();
            let oldStorage = self.storage.getValue();
            // Copy existing bytes
            for i in Int64(intLiteral: 0)..<oldStorage.len {
                newPtr.offset(by: i).write(oldStorage.ptr.offset(by: i).read())
            }
            // Free old buffer
            if oldStorage.cap > Int64(intLiteral: 0) {
                let oldLayout = Layout.array[UInt8](oldStorage.cap);
                allocator.deallocate(oldStorage.ptr.asRaw(), oldLayout)
            }
            self.storage.setValue(StringStorage(ptr: newPtr, len: oldStorage.len, cap: newCap))
        } else {
            lang.panic("String grow failed")
        }
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
        if otherLen == Int64(intLiteral: 0) {
            return
        }
        let myLen = self.len();
        self.makeUnique();
        self.grow(myLen + otherLen);
        var s = self.storage.getValue();
        let otherPtr = other.ptr();
        for i in Int64(intLiteral: 0)..<otherLen {
            s.ptr.offset(by: s.len).write(otherPtr.offset(by: i).read());
            s.len = s.len + Int64(intLiteral: 1)
        }
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
        self.makeUnique();
        self.grow(self.len() + utf8Len);
        var s = self.storage.getValue();
        // Encode to buffer
        let rawPtr: lang.ptr[lang.i8] = lang.cast_ptr[_, lang.i8](s.ptr.asRaw().raw);
        let written = encodeUtf8(c, rawPtr, at: s.len);
        s.len = s.len + written;
        self.storage.setValue(s)
    }

    /// Appends a raw byte to the buffer.
    ///
    /// **Unsafe** with respect to the UTF-8 invariant — the caller
    /// must ensure the resulting byte sequence is still valid UTF-8.
    /// Used primarily by substring helpers that copy whole UTF-8
    /// sequences in.
    ///
    /// # Safety
    ///
    /// The string must remain valid UTF-8 after the append; do not
    /// use this to inject continuation bytes into the middle of a
    /// sequence.
    public mutating func appendByte(byte: UInt8) {
        self.makeUnique();
        self.grow(self.len() + Int64(intLiteral: 1));
        var s = self.storage.getValue();
        s.ptr.offset(by: s.len).write(byte);
        s.len = s.len + Int64(intLiteral: 1);
        self.storage.setValue(s)
    }

    /// Truncates the string to length zero, keeping the allocated buffer.
    ///
    /// Capacity is unchanged, so this is the right primitive for
    /// reusing a buffer in a hot loop.
    public mutating func clear() {
        self.makeUnique();
        var s = self.storage.getValue();
        s.len = Int64(intLiteral: 0);
        self.storage.setValue(s)
    }

    // ========================================================================
    // SUBSTRINGS
    // ========================================================================

    /// Returns the substring spanning byte indices `[start, end)`.
    ///
    /// Out-of-range, inverted, or empty ranges return the empty
    /// string rather than panicking. The caller is responsible for
    /// ensuring the bounds fall on UTF-8 boundaries — see
    /// `BytesView.substring(checked:to:)` for a validated variant.
    ///
    /// # Examples
    ///
    /// ```
    /// "hello".substringBytes(from: 1, to: 4);   // "ell"
    /// "hello".substringBytes(from: 4, to: 1);   // ""    (inverted)
    /// "hello".substringBytes(from: 0, to: 99);  // ""    (out of range)
    /// ```
    public func substringBytes(from start: Int64, to end: Int64) -> String {
        let myLen = self.len();
        if start >= end or start < Int64(intLiteral: 0) or end > myLen {
            return String()
        }
        String.fromBytesUnchecked(self.ptr().offset(by: start), end - start)
    }

    // ========================================================================
    // SEARCHING
    // ========================================================================

    /// Returns true if `substring` appears anywhere in this string.
    ///
    /// Equivalent to `find(substring).isSome()`. The empty substring
    /// always matches.
    public func contains(substring: String) -> Bool {
        self.find(substring).isSome()
    }

    /// Returns true if any code point matches `predicate`.
    public func contains(matching predicate: (Char) -> Bool) -> Bool {
        self.find(matching: predicate).isSome()
    }

    /// Returns the byte offset of the first occurrence of `substring`, or `None`.
    ///
    /// Naïve byte-by-byte search; O(n·m) in the worst case where m is
    /// the substring length. The empty substring matches at offset
    /// `0`.
    ///
    /// # Examples
    ///
    /// ```
    /// "hello".find("ll");      // Some(2)
    /// "hello".find("xyz");     // None
    /// "hello".find("");        // Some(0)
    /// ```
    public func find(substring: String) -> Int64? {
        let subLen = substring.len();
        let myLen = self.len();
        if subLen == Int64(intLiteral: 0) {
            return .Some(Int64(intLiteral: 0))
        }
        if subLen > myLen {
            return .None
        }

        let myPtr = self.ptr();
        let subPtr = substring.ptr();
        var i: Int64 = Int64(intLiteral: 0);
        let lastStart = myLen - subLen;
        while i <= lastStart {
            var matches: Bool = true;
            var j: Int64 = Int64(intLiteral: 0);
            while j < subLen and matches {
                let a = myPtr.offset(by: i + j).read();
                let b = subPtr.offset(by: j).read();
                if a.equals(b) == false {
                    matches = false
                }
                j = j + Int64(intLiteral: 1)
            }
            if matches {
                return .Some(i)
            }
            i = i + Int64(intLiteral: 1)
        }
        return .None
    }

    /// Returns the byte offset of the first code point matching `predicate`, or `None`.
    ///
    /// Decodes UTF-8 as it scans so the predicate sees real `Char`s
    /// and the offset returned lands on a valid character boundary.
    public func find(matching predicate: (Char) -> Bool) -> Int64? {
        let myLen = self.len();
        let myPtr = self.ptr();
        var i: Int64 = Int64(intLiteral: 0);
        while i < myLen {
            let rawPtr: lang.ptr[lang.i8] = lang.cast_ptr[_, lang.i8](myPtr.asRaw().raw);
            let result = decodeUtf8(rawPtr, myLen, at: i);
            if let .Some(decoded) = result {
                if predicate(decoded.char) {
                    return .Some(i)
                }
                i = i + decoded.bytesConsumed
            } else {
                i = i + Int64(intLiteral: 1)
            }
        }
        return .None
    }

    /// Returns the byte offset of the *last* occurrence of `substring`, or `None`.
    ///
    /// Scans from the right but with the same naïve byte comparison
    /// as `find`. The empty substring matches at offset `byteCount`.
    ///
    /// # Examples
    ///
    /// ```
    /// "abcabc".reverseFind("abc");  // Some(3)
    /// "abcabc".reverseFind("");     // Some(6)
    /// ```
    public func reverseFind(substring: String) -> Int64? {
        let subLen = substring.len();
        let myLen = self.len();
        if subLen == Int64(intLiteral: 0) {
            return .Some(myLen)
        }
        if subLen > myLen {
            return .None
        }

        let myPtr = self.ptr();
        let subPtr = substring.ptr();
        var i: Int64 = myLen - subLen;
        while i >= Int64(intLiteral: 0) {
            var matches: Bool = true;
            var j: Int64 = Int64(intLiteral: 0);
            while j < subLen and matches {
                let a = myPtr.offset(by: i + j).read();
                let b = subPtr.offset(by: j).read();
                if a.equals(b) == false {
                    matches = false
                }
                j = j + Int64(intLiteral: 1)
            }
            if matches {
                return .Some(i)
            }
            i = i - Int64(intLiteral: 1)
        }
        return .None
    }

    /// Returns true if the string begins with `prefix`. O(prefix length).
    ///
    /// # Examples
    ///
    /// ```
    /// "hello".starts(with: "he");   // true
    /// "hello".starts(with: "lo");   // false
    /// ```
    public func starts(with prefix: String) -> Bool {
        let prefixLen = prefix.len();
        if prefixLen > self.len() {
            return false
        }
        let myPtr = self.ptr();
        let prefixPtr = prefix.ptr();
        var i: Int64 = Int64(intLiteral: 0);
        var matches: Bool = true;
        while i < prefixLen and matches {
            let a = myPtr.offset(by: i).read();
            let b = prefixPtr.offset(by: i).read();
            if a.equals(b) == false {
                matches = false
            }
            i = i + Int64(intLiteral: 1)
        }
        matches
    }

    /// Returns true if the string ends with `suffix`. O(suffix length).
    ///
    /// # Examples
    ///
    /// ```
    /// "hello".ends(with: "lo");  // true
    /// "hello".ends(with: "he");  // false
    /// ```
    public func ends(with suffix: String) -> Bool {
        let suffixLen = suffix.len();
        let myLen = self.len();
        if suffixLen > myLen {
            return false
        }
        let offset = myLen - suffixLen;
        let myPtr = self.ptr();
        let suffixPtr = suffix.ptr();
        var i: Int64 = Int64(intLiteral: 0);
        var matches: Bool = true;
        while i < suffixLen and matches {
            let a = myPtr.offset(by: offset + i).read();
            let b = suffixPtr.offset(by: i).read();
            if a.equals(b) == false {
                matches = false
            }
            i = i + Int64(intLiteral: 1)
        }
        matches
    }

    // ========================================================================
    // TRIMMING (Mutating)
    // ========================================================================

    /// Removes leading and trailing ASCII whitespace in place.
    ///
    /// Recognises space, tab, LF, CR — same set as `Char.isWhitespace`
    /// minus form feed (which `Char.isWhitespace` accepts but the
    /// trim helpers do not). For Unicode-aware trimming, use the
    /// `(matching:)` overloads with a custom predicate. Non-mutating
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
        let myLen = self.len();
        let myPtr = self.ptr();
        var realStart: Int64 = Int64(intLiteral: 0);
        var done: Bool = false;
        while realStart < myLen and done == false {
            let byte = myPtr.offset(by: realStart).read();
            let v: lang.i32 = lang.cast_i8_i32(byte.raw);
            let isWs = lang.i1_or(lang.i1_or(lang.i1_or(lang.i32_eq(v, 32), lang.i32_eq(v, 9)), lang.i32_eq(v, 10)), lang.i32_eq(v, 13));
            if Bool(boolLiteral: isWs) {
                realStart = realStart + Int64(intLiteral: 1)
            } else {
                done = true
            }
        }
        if realStart > Int64(intLiteral: 0) {
            self = self.substringBytes(from: realStart, to: myLen)
        }
    }

    /// Removes trailing ASCII whitespace in place.
    public mutating func trimEnd() {
        let myLen = self.len();
        let myPtr = self.ptr();
        var endPos: Int64 = myLen;
        var done: Bool = false;
        while endPos > Int64(intLiteral: 0) and done == false {
            let idx = endPos - Int64(intLiteral: 1);
            let byte = myPtr.offset(by: idx).read();
            let v: lang.i32 = lang.cast_i8_i32(byte.raw);
            let isWhitespace = lang.i1_or(lang.i1_or(lang.i1_or(lang.i32_eq(v, 32), lang.i32_eq(v, 9)), lang.i32_eq(v, 10)), lang.i32_eq(v, 13));
            if Bool(boolLiteral: isWhitespace) {
                endPos = endPos - Int64(intLiteral: 1)
            } else {
                done = true
            }
        }
        if endPos < myLen {
            self = self.substringBytes(from: Int64(intLiteral: 0), to: endPos)
        }
    }

    /// Removes leading and trailing code points matching `predicate`, in place.
    ///
    /// # Examples
    ///
    /// ```
    /// var s = "***hi***";
    /// s.trim(matching: |c| c == '*');
    /// s;  // "hi"
    /// ```
    public mutating func trim(matching predicate: (Char) -> Bool) {
        self.trimStart(matching: predicate);
        self.trimEnd(matching: predicate)
    }

    /// Removes leading code points matching `predicate`, in place.
    public mutating func trimStart(matching predicate: (Char) -> Bool) {
        let myLen = self.len();
        let myPtr = self.ptr();
        var realStart: Int64 = Int64(intLiteral: 0);
        var done: Bool = false;
        while realStart < myLen and done == false {
            let rawPtr: lang.ptr[lang.i8] = lang.cast_ptr[_, lang.i8](myPtr.asRaw().raw);
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
        if realStart > Int64(intLiteral: 0) {
            self = self.substringBytes(from: realStart, to: myLen)
        }
    }

    /// Removes trailing code points matching `predicate`, in place.
    ///
    /// Implemented by a forward scan that tracks the byte offset of
    /// the last non-matching character — UTF-8 is awkward to walk
    /// backwards without a side index.
    public mutating func trimEnd(matching predicate: (Char) -> Bool) {
        // For trimEnd, we need to scan from the end
        // This is tricky with UTF-8, so we scan forward and track valid end positions
        let myLen = self.len();
        let myPtr = self.ptr();
        var lastNonMatch: Int64 = Int64(intLiteral: 0);
        var i: Int64 = Int64(intLiteral: 0);
        while i < myLen {
            let rawPtr: lang.ptr[lang.i8] = lang.cast_ptr[_, lang.i8](myPtr.asRaw().raw);
            let result = decodeUtf8(rawPtr, myLen, at: i);
            if let .Some(decoded) = result {
                if predicate(decoded.char) == false {
                    lastNonMatch = i + decoded.bytesConsumed
                }
                i = i + decoded.bytesConsumed
            } else {
                i = i + Int64(intLiteral: 1)
            }
        }
        if lastNonMatch < myLen {
            self = self.substringBytes(from: Int64(intLiteral: 0), to: lastNonMatch)
        }
    }

    // ========================================================================
    // TRIMMING (Non-Mutating)
    // ========================================================================

    /// Returns a copy with leading and trailing ASCII whitespace removed.
    ///
    /// Non-mutating mirror of `trim()`.
    public func trimmed() -> String {
        self.trimmedStart().trimmedEnd()
    }

    /// Returns a copy with leading ASCII whitespace removed.
    public func trimmedStart() -> String {
        let myLen = self.len();
        let myPtr = self.ptr();
        var realStart: Int64 = Int64(intLiteral: 0);
        var done: Bool = false;
        while realStart < myLen and done == false {
            let byte = myPtr.offset(by: realStart).read();
            let v: lang.i32 = lang.cast_i8_i32(byte.raw);
            let isWs = lang.i1_or(lang.i1_or(lang.i1_or(lang.i32_eq(v, 32), lang.i32_eq(v, 9)), lang.i32_eq(v, 10)), lang.i32_eq(v, 13));
            if Bool(boolLiteral: isWs) {
                realStart = realStart + Int64(intLiteral: 1)
            } else {
                done = true
            }
        }
        self.substringBytes(from: realStart, to: myLen)
    }

    /// Returns a copy with trailing ASCII whitespace removed.
    public func trimmedEnd() -> String {
        let myLen = self.len();
        let myPtr = self.ptr();
        var endPos: Int64 = myLen;
        var done: Bool = false;
        while endPos > Int64(intLiteral: 0) and done == false {
            let idx = endPos - Int64(intLiteral: 1);
            let byte = myPtr.offset(by: idx).read();
            let v: lang.i32 = lang.cast_i8_i32(byte.raw);
            let isWhitespace = lang.i1_or(lang.i1_or(lang.i1_or(lang.i32_eq(v, 32), lang.i32_eq(v, 9)), lang.i32_eq(v, 10)), lang.i32_eq(v, 13));
            if Bool(boolLiteral: isWhitespace) {
                endPos = endPos - Int64(intLiteral: 1)
            } else {
                done = true
            }
        }
        self.substringBytes(from: Int64(intLiteral: 0), to: endPos)
    }

    /// Returns a copy with leading and trailing code points matching `predicate` removed.
    public func trimmed(matching predicate: (Char) -> Bool) -> String {
        self.trimmedStart(matching: predicate).trimmedEnd(matching: predicate)
    }

    /// Returns a copy with leading code points matching `predicate` removed.
    public func trimmedStart(matching predicate: (Char) -> Bool) -> String {
        let myLen = self.len();
        let myPtr = self.ptr();
        var realStart: Int64 = Int64(intLiteral: 0);
        var done: Bool = false;
        while realStart < myLen and done == false {
            let rawPtr: lang.ptr[lang.i8] = lang.cast_ptr[_, lang.i8](myPtr.asRaw().raw);
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
        self.substringBytes(from: realStart, to: myLen)
    }

    /// Returns a copy with trailing code points matching `predicate` removed.
    public func trimmedEnd(matching predicate: (Char) -> Bool) -> String {
        let myLen = self.len();
        let myPtr = self.ptr();
        var lastNonMatch: Int64 = Int64(intLiteral: 0);
        var i: Int64 = Int64(intLiteral: 0);
        while i < myLen {
            let rawPtr: lang.ptr[lang.i8] = lang.cast_ptr[_, lang.i8](myPtr.asRaw().raw);
            let result = decodeUtf8(rawPtr, myLen, at: i);
            if let .Some(decoded) = result {
                if predicate(decoded.char) == false {
                    lastNonMatch = i + decoded.bytesConsumed
                }
                i = i + decoded.bytesConsumed
            } else {
                i = i + Int64(intLiteral: 1)
            }
        }
        self.substringBytes(from: Int64(intLiteral: 0), to: lastNonMatch)
    }

    // ========================================================================
    // CASE CONVERSION (ASCII-only)
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
        self.makeUnique();
        let myLen = self.len();
        var s = self.storage.getValue();
        for i in Int64(intLiteral: 0)..<myLen {
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
        self.makeUnique();
        let myLen = self.len();
        var s = self.storage.getValue();
        for i in Int64(intLiteral: 0)..<myLen {
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

    /// Returns a copy with ASCII letters lowercased; non-ASCII bytes pass through unchanged.
    public func lowercasedAscii() -> String {
        let myLen = self.len();
        let myPtr = self.ptr();
        var result = String(capacity: myLen);
        for i in Int64(intLiteral: 0)..<myLen {
            let byte = myPtr.offset(by: i).read();
            let v: lang.i32 = lang.cast_i8_i32(byte.raw);
            // A-Z: 65-90 -> a-z: 97-122
            let isUppercase = lang.i1_and(lang.i32_signed_ge(v, 65), lang.i32_signed_le(v, 90));
            if Bool(boolLiteral: isUppercase) {
                result.appendByte(UInt8(raw: lang.cast_i32_i8(lang.i32_add(v, 32))))
            } else {
                result.appendByte(byte)
            }
        }
        result
    }

    /// Returns a copy with ASCII letters uppercased; non-ASCII bytes pass through unchanged.
    public func uppercasedAscii() -> String {
        let myLen = self.len();
        let myPtr = self.ptr();
        var result = String(capacity: myLen);
        for i in Int64(intLiteral: 0)..<myLen {
            let byte = myPtr.offset(by: i).read();
            let v: lang.i32 = lang.cast_i8_i32(byte.raw);
            // a-z: 97-122 -> A-Z: 65-90
            let isLowercase = lang.i1_and(lang.i32_signed_ge(v, 97), lang.i32_signed_le(v, 122));
            if Bool(boolLiteral: isLowercase) {
                result.appendByte(UInt8(raw: lang.cast_i32_i8(lang.i32_sub(v, 32))))
            } else {
                result.appendByte(byte)
            }
        }
        result
    }

    // ========================================================================
    // CASE CONVERSION (Unicode)
    // ========================================================================

    /// Replaces this string with its lowercase form using full Unicode case mapping.
    ///
    /// Locale-independent. Handles multi-character expansions
    /// (rare in lowercasing). Implemented as `self = self.lowercased()`,
    /// so a transient new buffer is allocated.
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

    /// Returns the lowercase form using full Unicode case mapping.
    ///
    /// Two fast paths: an all-ASCII string with no uppercase letters
    /// is returned cloned (no allocation beyond the COW share); an
    /// all-ASCII string with uppercase letters routes to
    /// `lowercasedAscii`. The slow path uses the Unicode tables and
    /// honours multi-char expansions.
    ///
    /// # Examples
    ///
    /// ```
    /// "Hello".lowercased();      // "hello"
    /// "\u{0130}".lowercased();   // "i\u{0307}" (Turkish dotted I expansion)
    /// ```
    public func lowercased() -> String {
        // Fast path: check if all ASCII
        let myLen = self.len();
        var allAscii = true;
        var hasUpperAscii = false;
        for i in Int64(intLiteral: 0)..<myLen {
            let byte = self.ptr().offset(by: i).read();
            if byte > 127 {
                allAscii = false
            }
            if byte >= 65 and byte <= 90 {
                hasUpperAscii = true
            }
        }

        if allAscii {
            // ASCII-only fast path
            if hasUpperAscii == false {
                return self.clone()
            }
            return self.lowercasedAscii()
        }

        // Full Unicode path
        var result = String();
        for c in self.chars.iter() {
            if unicode.hasLowercaseExpansion(c) {
                result.append(unicode.lowercaseExpansion(c))
            } else {
                result.appendChar(unicode.toLowercase(c))
            }
        }
        result
    }

    /// Returns the uppercase form using full Unicode case mapping.
    ///
    /// Symmetric to `lowercased`; the same ASCII fast paths apply.
    /// Multi-char expansions (e.g. `ß` → `SS`) are honoured.
    ///
    /// # Examples
    ///
    /// ```
    /// "hello".uppercased();      // "HELLO"
    /// "stra\u{00DF}e".uppercased();  // "STRASSE" (ß expands to SS)
    /// ```
    public func uppercased() -> String {
        // Fast path: check if all ASCII
        let myLen = self.len();
        var allAscii = true;
        var hasLowerAscii = false;
        for i in Int64(intLiteral: 0)..<myLen {
            let byte = self.ptr().offset(by: i).read();
            if byte > 127 {
                allAscii = false
            }
            if byte >= 97 and byte <= 122 {
                hasLowerAscii = true
            }
        }

        if allAscii {
            // ASCII-only fast path
            if hasLowerAscii == false {
                return self.clone()
            }
            return self.uppercasedAscii()
        }

        // Full Unicode path
        var result = String();
        for c in self.chars.iter() {
            if unicode.hasUppercaseExpansion(c) {
                result.append(unicode.uppercaseExpansion(c))
            } else {
                result.appendChar(unicode.toUppercase(c))
            }
        }
        result
    }

    /// Returns the titlecase form using full Unicode case mapping.
    ///
    /// Word boundaries are detected by `Char.isWhitespace`; the first
    /// non-space character of each run is titlecased and the rest
    /// lowercased. This is a coarse model — it doesn't handle
    /// hyphenated names or apostrophe-internal capitals — but works
    /// for plain whitespace-separated text.
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
            if c.isWhitespace() {
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

    /// Compares two strings for equality after Unicode case folding.
    ///
    /// Walks both `chars` iterators in lockstep, folding each pair of
    /// code points before comparing. Note: this is not normalization
    /// aware — `é` (`U+00E9`) and `e\u{0301}` are still considered
    /// different. Normalize both sides first if you need that.
    ///
    /// # Examples
    ///
    /// ```
    /// "Hello".equalsCaseInsensitive("HELLO");  // true
    /// "Hello".equalsCaseInsensitive("World");  // false
    /// ```
    public func equalsCaseInsensitive(other: String) -> Bool {
        // Compare case-folded versions
        var selfIter = self.chars.iter();
        var otherIter = other.chars.iter();

        while true {
            let selfChar = selfIter.next();
            let otherChar = otherIter.next();

            match (selfChar, otherChar) {
                (.None, .None) => { return true },
                (.Some(sc), .Some(oc)) => {
                    // Compare case-folded characters
                    let foldedSelf = unicode.caseFold(sc);
                    let foldedOther = unicode.caseFold(oc);
                    if foldedSelf.equals(foldedOther) == false {
                        return false
                    }
                },
                _ => { return false }
            }
        }
        // Unreachable
        false
    }

    // ========================================================================
    // REPLACEMENT (Mutating)
    // ========================================================================

    /// Replaces every occurrence of `pattern` with `replacement`, in place.
    ///
    /// Allocates a fresh string under the hood; the in-place form is
    /// for ergonomics, not buffer reuse.
    public mutating func replace(pattern: String, with replacement: String) {
        self = self.replaced(pattern, with: replacement)
    }

    // ========================================================================
    // REPLACEMENT (Non-Mutating)
    // ========================================================================

    /// Returns a copy with every occurrence of `pattern` replaced by `replacement`.
    ///
    /// Empty `pattern` is a no-op (returns a clone). Searches greedily
    /// from the left and skips past each replacement so substituted
    /// text is not re-matched.
    ///
    /// # Examples
    ///
    /// ```
    /// "hello world".replaced("o", with: "0");      // "hell0 w0rld"
    /// "abcabc".replaced("ab", with: "ABCD");       // "ABCDcABCDc"
    /// ```
    public func replaced(pattern: String, with replacement: String) -> String {
        let patternLen = pattern.len();
        if patternLen == Int64(intLiteral: 0) {
            return self.clone()
        }

        let myLen = self.len();
        let myPtr = self.ptr();
        let patternPtr = pattern.ptr();
        var result = String();
        var i: Int64 = Int64(intLiteral: 0);

        while i < myLen {
            // Check for pattern match
            var matches: Bool = true;
            if i + patternLen <= myLen {
                for j in Int64(intLiteral: 0)..<patternLen {
                    let a = myPtr.offset(by: i + j).read();
                    let b = patternPtr.offset(by: j).read();
                    if a.equals(b) == false {
                        matches = false
                    }
                }
            } else {
                matches = false
            }

            if matches {
                result.append(replacement);
                i = i + patternLen
            } else {
                result.appendByte(myPtr.offset(by: i).read());
                i = i + Int64(intLiteral: 1)
            }
        }
        result
    }

    // ========================================================================
    // SPLITTING
    // ========================================================================

    /// Returns an iterator that splits this string on `separator` (byte-exact).
    ///
    /// The empty separator is special-cased to split per code point.
    /// See `SplitIterator` for the iteration shape.
    ///
    /// # Examples
    ///
    /// ```
    /// var parts = Array[String]();
    /// for p in "a,b,c".split(separator: ",") { parts.append(p); }
    /// parts.count;  // 3
    /// ```
    public func split(separator: String) -> SplitIterator {
        SplitIterator(
            ptr: self.ptr(),
            length: self.len(),
            sepPtr: separator.ptr(),
            sepLen: separator.len()
        )
    }

    /// Returns an iterator that splits at every code point matching `predicate`.
    ///
    /// The matching characters are not included in any segment.
    ///
    /// # Examples
    ///
    /// ```
    /// var parts = Array[String]();
    /// for p in "a 1 b 2 c".split(matching: |c| c.isDigit() or c.isWhitespace()) {
    ///     if p.isEmpty == false { parts.append(p); }
    /// }
    /// // parts: ["a", "b", "c"]
    /// ```
    public func split(matching predicate: (Char) -> Bool) -> SplitWhereIterator {
        SplitWhereIterator(
            ptr: self.ptr(),
            length: self.len(),
            predicate: predicate
        )
    }

    // ========================================================================
    // REPEATING & PADDING
    // ========================================================================

    /// Returns this string concatenated with itself `count` times.
    ///
    /// Non-positive `count` returns the empty string. Sizes the
    /// result buffer for the exact final length to avoid growth.
    ///
    /// # Examples
    ///
    /// ```
    /// "ab".repeated(count: 3);  // "ababab"
    /// "ab".repeated(count: 0);  // ""
    /// ```
    public func repeated(count: Int64) -> String {
        if count <= Int64(intLiteral: 0) {
            return String()
        }
        let myLen = self.len();
        var result = String(capacity: myLen * count);
        for i in Int64(intLiteral: 0)..<count {
            result.append(self)
        }
        result
    }

    /// Returns the string padded at the start with `char` so the total *code-point* count is `length`.
    ///
    /// If the string is already at least `length` code points long,
    /// returns a clone. Compare with `pad(end:with:)` for trailing
    /// padding.
    ///
    /// # Examples
    ///
    /// ```
    /// "42".pad(start: 5, with: '0');  // "00042"
    /// ```
    public func pad(start length: Int64, with char: Char) -> String {
        let currentLen = self.count;
        if currentLen >= length {
            return self.clone()
        }
        let paddingCount = length - currentLen;
        var result = String(capacity: self.len() + paddingCount * char.utf8Length());
        for i in Int64(intLiteral: 0)..<paddingCount {
            result.appendChar(char)
        }
        result.append(self);
        result
    }

    /// Returns the string padded at the end with `char` so the total *code-point* count is `length`.
    ///
    /// # Examples
    ///
    /// ```
    /// "42".pad(end: 5, with: '.');  // "42..."
    /// ```
    public func pad(end length: Int64, with char: Char) -> String {
        let currentLen = self.count;
        if currentLen >= length {
            return self.clone()
        }
        let paddingCount = length - currentLen;
        var result = String(capacity: self.len() + paddingCount * char.utf8Length());
        result.append(self);
        for i in Int64(intLiteral: 0)..<paddingCount {
            result.appendChar(char)
        }
        result
    }

    // ========================================================================
    // ITERATION
    // ========================================================================

    /// Returns a `StringIterator` over the code points starting at byte 0.
    ///
    /// Required by `Iterable`. Each call returns a fresh iterator;
    /// the string itself is reusable.
    public func iter() -> StringIterator {
        StringIterator(ptr: self.ptr(), length: self.len())
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
    /// "abc".equals("abc");  // true
    /// "abc".equals("ABC");  // false
    /// ```
    public func equals(other: String) -> Bool {
        let myLen = self.len();
        let otherLen = other.len();
        if myLen != otherLen {
            return false
        }
        let myPtr = self.ptr();
        let otherPtr = other.ptr();
        var equal: Bool = true;
        for i in Int64(intLiteral: 0)..<myLen {
            let a = myPtr.offset(by: i).read();
            let b = otherPtr.offset(by: i).read();
            if a.equals(b) == false {
                equal = false
            }
        }
        equal
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

        let myPtr = self.ptr();
        let otherPtr = other.ptr();
        for i in Int64(intLiteral: 0)..<minLen {
            let a = myPtr.offset(by: i).read();
            let b = otherPtr.offset(by: i).read();
            let cmp = a.compare(b);
            let eql: Ordering = .Equal;
            if cmp.equals(eql) == false {
                return cmp
            }
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
        hasher.write(Slice(pointer: self.ptr(), count: self.len()))
    }

    /// Returns a shallow clone — storage is shared until either side mutates.
    ///
    /// O(1). Mutation triggers `makeUnique` which performs a deep
    /// copy.
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
    /// "test".format(options: opts);   // "test      "
    /// opts.alignment = .Right;
    /// "test".format(options: opts);   // "      test"
    /// opts.alignment = .Center;
    /// "test".format(options: opts);   // "   test   "
    /// ```
    public func format(options: FormatOptions = FormatOptions.default()) -> String {
        // Apply width and alignment padding
        if let .Some(width) = options.width {
            let currentLen = self.count;
            if width > currentLen {
                let padding = width - currentLen;
                var padLeft: Int64 = 0;
                var padRight: Int64 = 0;

                if options.alignment == .Left {
                    padRight = padding
                } else if options.alignment == .Right {
                    padLeft = padding
                } else {
                    // Center
                    padLeft = padding / 2;
                    padRight = padding - padLeft
                }

                var result = String();
                while padLeft > 0 {
                    result.appendChar(options.fill);
                    padLeft = padLeft - 1
                }
                result.append(self);
                while padRight > 0 {
                    result.appendChar(options.fill);
                    padRight = padRight - 1
                }
                return result
            }
        }
        self.clone()
    }
}
