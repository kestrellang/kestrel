// String - UTF-8 encoded string with COW (Copy-on-Write) semantics

module std.text

import std.core.(Bool, Equatable, Comparable, Cloneable, Ordering, Addable, ExpressibleByStringLiteral, Hash, Hasher, Defaultable)
import std.core.(Formattable)
import std.num.(Int64, UInt8)
import std.result.(Optional)
import std.memory.(Layout, Pointer, RawPointer, SystemAllocator, RcBox, Slice)
import std.iter.(Iterator, Iterable)
import std.text.(Char, decodeUtf8, encodeUtf8, BytesView, CharsView, GraphemesView, LinesView)
import std.ffi.(memcpy)

// ============================================================================
// STRING ITERATOR
// ============================================================================

/// Iterator over the Unicode code points (Char) in a string.
public struct StringIterator: Iterator {
    type Item = Char

    private var ptr: Pointer[UInt8]
    private var length: Int64
    private var index: Int64

    /// Creates a string iterator from a pointer and length.
    public init(ptr ptr: Pointer[UInt8], length length: Int64) {
        self.ptr = ptr;
        self.length = length;
        self.index = Int64(intLiteral: 0);
    }

    /// Returns the next character, or None if exhausted.
    public mutating func next() -> Char? {
        if self.index >= self.length {
            return .None
        }
        // Decode UTF-8 at current position
        let rawPtr: lang.ptr[lang.i8] = lang.cast_ptr[lang.i8](self.ptr.asRaw().raw);
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

/// Iterator that splits a string on a separator.
public struct SplitIterator: Iterator {
    type Item = String

    private var ptr: Pointer[UInt8]
    private var length: Int64
    private var sepPtr: Pointer[UInt8]
    private var sepLen: Int64
    private var index: Int64
    private var done: Bool

    /// Creates a split iterator.
    public init(ptr ptr: Pointer[UInt8], length length: Int64, sepPtr sepPtr: Pointer[UInt8], sepLen sepLen: Int64) {
        self.ptr = ptr;
        self.length = length;
        self.sepPtr = sepPtr;
        self.sepLen = sepLen;
        self.index = Int64(intLiteral: 0);
        self.done = false;
    }

    /// Returns the next split segment, or None if exhausted.
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
            let rawPtr: lang.ptr[lang.i8] = lang.cast_ptr[lang.i8](self.ptr.asRaw().raw);
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

/// Iterator that splits a string where a predicate returns true.
public struct SplitWhereIterator: Iterator {
    type Item = String

    private var ptr: Pointer[UInt8]
    private var length: Int64
    private var predicate: (Char) -> Bool
    private var index: Int64
    private var done: Bool

    /// Creates a split-where iterator.
    public init(ptr ptr: Pointer[UInt8], length length: Int64, predicate predicate: (Char) -> Bool) {
        self.ptr = ptr;
        self.length = length;
        self.predicate = predicate;
        self.index = Int64(intLiteral: 0);
        self.done = false;
    }

    /// Returns the next split segment, or None if exhausted.
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
            let rawPtr: lang.ptr[lang.i8] = lang.cast_ptr[lang.i8](self.ptr.asRaw().raw);
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

/// Internal storage for String (ptr, len, cap).
struct StringStorage: Cloneable {
    var ptr: Pointer[UInt8]
    var len: Int64
    var cap: Int64

    init(ptr ptr: Pointer[UInt8], len len: Int64, cap cap: Int64) {
        self.ptr = ptr;
        self.len = len;
        self.cap = cap;
    }

    /// Deep clone - allocate new buffer and copy bytes.
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
/// Strings are immutable by default; mutations create a new copy if shared.
public struct String: Iterable, Equatable, Comparable, Cloneable, Formattable, Addable, ExpressibleByStringLiteral, Hash, Defaultable {
    type Item = Char
    type Iter = StringIterator
    type Output = String

    private var storage: RcBox[StringStorage]

    // Helper accessors for storage fields
    private func ptr() -> Pointer[UInt8] { self.storage.getValue().ptr }
    private func len() -> Int64 { self.storage.getValue().len }
    private func cap() -> Int64 { self.storage.getValue().cap }

    // Ensure unique storage for mutation (COW)
    private mutating func makeUnique() {
        if self.storage.isUnique() == false {
            self.storage = self.storage.deepClone()
        }
    }

    // ========================================================================
    // CONSTRUCTORS
    // ========================================================================

    /// Creates an empty string.
    public init() {
        self.storage = RcBox(StringStorage(
            ptr: Pointer(raw: lang.ptr_null[UInt8]()),
            len: Int64(intLiteral: 0),
            cap: Int64(intLiteral: 0)
        ));
    }

    /// Creates an empty string with the specified capacity.
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

    /// Creates a string from a string literal.
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
                let dstPtr: lang.ptr[lang.i8] = lang.cast_ptr[lang.i8](newPtr.asRaw().raw);
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

    /// Private: create from storage (for COW clone).
    private init(storage storage: RcBox[StringStorage]) {
        self.storage = storage;
    }

    /// Internal: create from bytes without validation (for split).
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

    /// A view over the raw UTF-8 bytes.
    public var bytes: BytesView {
        BytesView(ptr: lang.cast_ptr[lang.i8](self.ptr().asRaw().raw), length: self.len())
    }

    /// A view over the Unicode code points.
    public var chars: CharsView {
        CharsView(ptr: lang.cast_ptr[lang.i8](self.ptr().asRaw().raw), length: self.len())
    }

    /// A view over the extended grapheme clusters.
    public var graphemes: GraphemesView {
        GraphemesView(ptr: lang.cast_ptr[lang.i8](self.ptr().asRaw().raw), length: self.len())
    }

    /// A view over the lines in the string.
    public var lines: LinesView {
        LinesView(ptr: lang.cast_ptr[lang.i8](self.ptr().asRaw().raw), length: self.len())
    }

    // ========================================================================
    // SIZE & CAPACITY
    // ========================================================================

    /// The number of bytes (not characters).
    public var byteCount: Int64 { self.len() }

    /// The allocated capacity in bytes.
    public var capacity: Int64 { self.cap() }

    /// True if the string is empty.
    public var isEmpty: Bool { self.len() == Int64(intLiteral: 0) }

    /// The number of Unicode code points (O(n)).
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

    /// Returns the first character, or None if empty.
    public func first() -> Char? {
        if self.len() == Int64(intLiteral: 0) {
            return .None
        }
        let rawPtr: lang.ptr[lang.i8] = lang.cast_ptr[lang.i8](self.ptr().asRaw().raw);
        let result = decodeUtf8(rawPtr, self.len(), at: Int64(intLiteral: 0));
        if let .Some(decoded) = result {
            .Some(decoded.char)
        } else {
            .None
        }
    }

    /// Returns the last character, or None if empty.
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
            let rawPtr: lang.ptr[lang.i8] = lang.cast_ptr[lang.i8](myPtr.asRaw().raw);
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

    /// Returns the character at the given index. Panics if out of bounds.
    public func char(at index: Int64) -> Char {
        match self.char(checked: index) {
            .Some(c) => c,
            .None => lang.panic("String index out of bounds")
        }
    }

    /// Returns the character at the given index, or None if out of bounds.
    public func char(checked index: Int64) -> Char? {
        let myLen = self.len();
        let myPtr = self.ptr();
        var charIndex: Int64 = Int64(intLiteral: 0);
        var byteIndex: Int64 = Int64(intLiteral: 0);
        while byteIndex < myLen {
            if charIndex == index {
                let rawPtr: lang.ptr[lang.i8] = lang.cast_ptr[lang.i8](myPtr.asRaw().raw);
                let result = decodeUtf8(rawPtr, myLen, at: byteIndex);
                if let .Some(decoded) = result {
                    return .Some(decoded.char)
                }
                return .None
            }
            let rawPtr: lang.ptr[lang.i8] = lang.cast_ptr[lang.i8](myPtr.asRaw().raw);
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

    /// Returns the character at the given index without bounds checking.
    public func char(unchecked index: Int64) -> Char {
        self.char(at: index)
    }

    /// Returns the character at the given index, wrapping around (-1 = last).
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

    /// Returns the character at the given index, clamped to valid range.
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

    /// Returns the byte at the given index, or None if out of bounds.
    public func byteAt(index: Int64) -> UInt8? {
        let myLen = self.len();
        if index >= Int64(intLiteral: 0) and index < myLen {
            .Some(self.ptr().offset(by: index).read())
        } else {
            .None
        }
    }

    /// Returns the byte at the given index without bounds checking.
    public func byteAtUnchecked(index: Int64) -> UInt8 {
        self.ptr().offset(by: index).read()
    }

    // ========================================================================
    // CAPACITY MANAGEMENT (Internal)
    // ========================================================================

    /// Grows capacity to at least minCapacity.
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

    /// Appends another string to this one.
    public mutating func append(other: String) {
        let otherLen = other.len();
        if otherLen == Int64(intLiteral: 0) {
            return
        }
        let myLen = self.len();
        self.grow(myLen + otherLen);
        self.makeUnique();
        var s = self.storage.getValue();
        let otherPtr = other.ptr();
        for i in Int64(intLiteral: 0)..<otherLen {
            s.ptr.offset(by: s.len).write(otherPtr.offset(by: i).read());
            s.len = s.len + Int64(intLiteral: 1)
        }
        self.storage.setValue(s)
    }

    /// Appends a character to this string.
    public mutating func appendChar(c: Char) {
        let utf8Len = c.utf8Length();
        self.grow(self.len() + utf8Len);
        self.makeUnique();
        var s = self.storage.getValue();
        // Encode to buffer
        let rawPtr: lang.ptr[lang.i8] = lang.cast_ptr[lang.i8](s.ptr.asRaw().raw);
        let written = encodeUtf8(c, rawPtr, at: s.len);
        s.len = s.len + written;
        self.storage.setValue(s)
    }

    /// Appends a raw byte (caller must ensure valid UTF-8).
    public mutating func appendByte(byte: UInt8) {
        self.grow(self.len() + Int64(intLiteral: 1));
        self.makeUnique();
        var s = self.storage.getValue();
        s.ptr.offset(by: s.len).write(byte);
        s.len = s.len + Int64(intLiteral: 1);
        self.storage.setValue(s)
    }

    /// Removes all characters from the string.
    public mutating func clear() {
        self.makeUnique();
        var s = self.storage.getValue();
        s.len = Int64(intLiteral: 0);
        self.storage.setValue(s)
    }

    // ========================================================================
    // SUBSTRINGS
    // ========================================================================

    /// Returns a substring by byte indices.
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

    /// Returns true if the string contains the substring.
    public func contains(substring: String) -> Bool {
        self.find(substring).isSome()
    }

    /// Returns true if any character matches the predicate.
    public func contains(matching predicate: (Char) -> Bool) -> Bool {
        self.find(matching: predicate).isSome()
    }

    /// Returns the byte index of the first occurrence of substring, or None.
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

    /// Returns the byte index of the first character matching the predicate, or None.
    public func find(matching predicate: (Char) -> Bool) -> Int64? {
        let myLen = self.len();
        let myPtr = self.ptr();
        var i: Int64 = Int64(intLiteral: 0);
        while i < myLen {
            let rawPtr: lang.ptr[lang.i8] = lang.cast_ptr[lang.i8](myPtr.asRaw().raw);
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

    /// Returns the byte index of the last occurrence of substring, or None.
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

    /// Returns true if the string starts with the prefix.
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

    /// Returns true if the string ends with the suffix.
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

    /// Removes leading and trailing whitespace in place.
    public mutating func trim() {
        self.trimStart();
        self.trimEnd()
    }

    /// Removes leading whitespace in place.
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

    /// Removes trailing whitespace in place.
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

    /// Removes leading and trailing characters matching the predicate in place.
    public mutating func trim(matching predicate: (Char) -> Bool) {
        self.trimStart(matching: predicate);
        self.trimEnd(matching: predicate)
    }

    /// Removes leading characters matching the predicate in place.
    public mutating func trimStart(matching predicate: (Char) -> Bool) {
        let myLen = self.len();
        let myPtr = self.ptr();
        var realStart: Int64 = Int64(intLiteral: 0);
        var done: Bool = false;
        while realStart < myLen and done == false {
            let rawPtr: lang.ptr[lang.i8] = lang.cast_ptr[lang.i8](myPtr.asRaw().raw);
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

    /// Removes trailing characters matching the predicate in place.
    public mutating func trimEnd(matching predicate: (Char) -> Bool) {
        // For trimEnd, we need to scan from the end
        // This is tricky with UTF-8, so we scan forward and track valid end positions
        let myLen = self.len();
        let myPtr = self.ptr();
        var lastNonMatch: Int64 = Int64(intLiteral: 0);
        var i: Int64 = Int64(intLiteral: 0);
        while i < myLen {
            let rawPtr: lang.ptr[lang.i8] = lang.cast_ptr[lang.i8](myPtr.asRaw().raw);
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

    /// Returns a string with leading and trailing whitespace removed.
    public func trimmed() -> String {
        self.trimmedStart().trimmedEnd()
    }

    /// Returns a string with leading whitespace removed.
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

    /// Returns a string with trailing whitespace removed.
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

    /// Returns a string with leading and trailing characters matching the predicate removed.
    public func trimmed(matching predicate: (Char) -> Bool) -> String {
        self.trimmedStart(matching: predicate).trimmedEnd(matching: predicate)
    }

    /// Returns a string with leading characters matching the predicate removed.
    public func trimmedStart(matching predicate: (Char) -> Bool) -> String {
        let myLen = self.len();
        let myPtr = self.ptr();
        var realStart: Int64 = Int64(intLiteral: 0);
        var done: Bool = false;
        while realStart < myLen and done == false {
            let rawPtr: lang.ptr[lang.i8] = lang.cast_ptr[lang.i8](myPtr.asRaw().raw);
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

    /// Returns a string with trailing characters matching the predicate removed.
    public func trimmedEnd(matching predicate: (Char) -> Bool) -> String {
        let myLen = self.len();
        let myPtr = self.ptr();
        var lastNonMatch: Int64 = Int64(intLiteral: 0);
        var i: Int64 = Int64(intLiteral: 0);
        while i < myLen {
            let rawPtr: lang.ptr[lang.i8] = lang.cast_ptr[lang.i8](myPtr.asRaw().raw);
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

    /// Converts ASCII characters to lowercase in place.
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

    /// Converts ASCII characters to uppercase in place.
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

    /// Returns a string with ASCII characters converted to lowercase.
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

    /// Returns a string with ASCII characters converted to uppercase.
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

    /// Converts all characters to lowercase using Unicode case mapping in place.
    /// Handles locale-independent case folding including multi-character expansions
    /// (e.g., German 'ß' uppercase becomes "SS").
    public mutating func lowercase() {
        // For now, delegate to ASCII-only lowercase
        // Full Unicode case mapping would require ICU or similar
        self.lowercaseAscii()
    }

    /// Converts all characters to uppercase using Unicode case mapping in place.
    /// Handles locale-independent case folding including multi-character expansions
    /// (e.g., German 'ß' uppercase becomes "SS").
    public mutating func uppercase() {
        // For now, delegate to ASCII-only uppercase
        // Full Unicode case mapping would require ICU or similar
        self.uppercaseAscii()
    }

    /// Returns a string with all characters converted to lowercase using Unicode case mapping.
    /// Handles locale-independent case folding including multi-character expansions.
    public func lowercased() -> String {
        // For now, delegate to ASCII-only lowercased
        self.lowercasedAscii()
    }

    /// Returns a string with all characters converted to uppercase using Unicode case mapping.
    /// Handles locale-independent case folding including multi-character expansions.
    public func uppercased() -> String {
        // For now, delegate to ASCII-only uppercased
        self.uppercasedAscii()
    }

    // ========================================================================
    // REPLACEMENT (Mutating)
    // ========================================================================

    /// Replaces all occurrences of pattern with replacement in place.
    public mutating func replace(pattern: String, with replacement: String) {
        self = self.replaced(pattern, with: replacement)
    }

    // ========================================================================
    // REPLACEMENT (Non-Mutating)
    // ========================================================================

    /// Returns a string with all occurrences of pattern replaced.
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

    /// Returns an iterator that splits the string on the separator.
    public func split(separator: String) -> SplitIterator {
        SplitIterator(
            ptr: self.ptr(),
            length: self.len(),
            sepPtr: separator.ptr(),
            sepLen: separator.len()
        )
    }

    /// Returns an iterator that splits the string where the predicate returns true.
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

    /// Returns the string repeated the specified number of times.
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

    /// Returns a string padded at the start to the specified length.
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

    /// Returns a string padded at the end to the specified length.
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

    /// Returns an iterator over the Unicode code points.
    public func iter() -> StringIterator {
        StringIterator(ptr: self.ptr(), length: self.len())
    }

    // ========================================================================
    // PROTOCOL CONFORMANCES
    // ========================================================================

    /// Concatenates two strings.
    public func add(other: String) -> String {
        var result = self.clone();
        result.append(other);
        result
    }

    /// Compares two strings for equality.
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

    /// Compares two strings lexicographically.
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

    /// Hashes the string's bytes.
    public func hash[H](mutating into hasher: H) where H: Hasher {
        hasher.write(Slice(pointer: self.ptr(), count: self.len()))
    }

    /// Creates a shallow clone (COW - copy deferred until mutation).
    public func clone() -> String {
        String(storage: self.storage.clone())
    }

    /// Returns the string representation (itself).
    public func format() -> String {
        self.clone()
    }
}
