// String - UTF-8 encoded string with COW (Copy-on-Write) semantics

module std.text

import std.core.(Bool, Equatable, Comparable, Cloneable, Formattable, Ordering, Addable, ExpressibleByStringLiteral)
import std.num.(Int64, UInt8)
import std.result.(Optional)
import std.memory.(Layout, Pointer, RawPointer, SystemAllocator, RcBox)
import std.iter.(Iterator, Iterable)
import std.text.(Char, decodeUtf8, encodeUtf8)
import std.ffi.(memcpy)

// StringIterator - iterates over chars
public struct StringIterator: Iterator {
    type Item = Char

    private var ptr: Pointer[UInt8]
    private var length: Int64
    private var index: Int64

    public init(ptr ptr: Pointer[UInt8], length length: Int64) {
        self.ptr = ptr;
        self.length = length;
        self.index = Int64(intLiteral: 0);
    }

    public mutating func next() -> Optional[Char] {
        if self.index >= self.length {
            return .None
        }
        // Decode UTF-8 at current position
        let rawPtr: lang.ptr[lang.i8] = lang.cast_ptr[lang.i8](self.ptr.asRaw().raw);
        let result = decodeUtf8(rawPtr, self.length, at: self.index);
        if result.isSome() {
            let decoded = result.unwrap();
            self.index = self.index + decoded.bytesConsumed;
            .Some(decoded.char)
        } else {
            // Invalid UTF-8, skip one byte
            self.index = self.index + Int64(intLiteral: 1);
            .None
        }
    }
}

// SplitIterator - splits string on separator
public struct SplitIterator: Iterator {
    type Item = String

    private var ptr: Pointer[UInt8]
    private var length: Int64
    private var sepPtr: Pointer[UInt8]
    private var sepLen: Int64
    private var index: Int64
    private var done: Bool

    public init(ptr ptr: Pointer[UInt8], length length: Int64, sepPtr sepPtr: Pointer[UInt8], sepLen sepLen: Int64) {
        self.ptr = ptr;
        self.length = length;
        self.sepPtr = sepPtr;
        self.sepLen = sepLen;
        self.index = Int64(intLiteral: 0);
        self.done = false;
    }

    public mutating func next() -> Optional[String] {
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
            if result.isSome() {
                let decoded = result.unwrap();
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

// StringStorage - internal storage for String (ptr, len, cap)
struct StringStorage: Cloneable {
    var ptr: Pointer[UInt8]
    var len: Int64
    var cap: Int64

    init(ptr ptr: Pointer[UInt8], len len: Int64, cap cap: Int64) {
        self.ptr = ptr;
        self.len = len;
        self.cap = cap;
    }

    // Deep clone - allocate new buffer and copy bytes
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
        if result.isSome() {
            let newPtr = result.unwrap().cast[UInt8]();
            // Copy bytes
            var i: Int64 = Int64(intLiteral: 0);
            while i < self.len {
                newPtr.offset(by: i).write(self.ptr.offset(by: i).read());
                i = i + Int64(intLiteral: 1)
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

// String - UTF-8 encoded, dynamically sized string with COW semantics
public struct String: Iterable, Equatable, Comparable, Cloneable, Formattable, Addable, ExpressibleByStringLiteral {
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

    // Create empty string
    public init() {
        self.storage = RcBox(StringStorage(
            ptr: Pointer(raw: lang.ptr_null[UInt8]()),
            len: Int64(intLiteral: 0),
            cap: Int64(intLiteral: 0)
        ));
    }

    // Create with capacity
    public init(capacity capacity: Int64) {
        if capacity > Int64(intLiteral: 0) {
            let layout = Layout.array[UInt8](capacity);
            var allocator = SystemAllocator();
            let result = allocator.allocate(layout);
            if result.isSome() {
                self.storage = RcBox(StringStorage(
                    ptr: result.unwrap().cast[UInt8](),
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

    // ExpressibleByStringLiteral
    public init(stringLiteral ptr: lang.ptr[lang.i8], length: lang.i64) {
        if lang.i64_signed_gt(length, 0) {
            let byteCount = Int64(intLiteral: length);
            let layout = Layout.array[UInt8](byteCount);
            var allocator = SystemAllocator();
            let result = allocator.allocate(layout);
            if result.isSome() {
                let newPtr = result.unwrap().cast[UInt8]();
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

    // Private: create from storage (for COW clone)
    private init(storage storage: RcBox[StringStorage]) {
        self.storage = storage;
    }

    // Internal: create from bytes without validation (for split)
    static func fromBytesUnchecked(ptr: Pointer[UInt8], count: Int64) -> String {
        if count == Int64(intLiteral: 0) {
            return String()
        }
        let layout = Layout.array[UInt8](count);
        var allocator = SystemAllocator();
        let result = allocator.allocate(layout);
        if result.isSome() {
            let newPtr = result.unwrap().cast[UInt8]();
            // Copy bytes
            var i: Int64 = Int64(intLiteral: 0);
            while i < count {
                newPtr.offset(by: i).write(ptr.offset(by: i).read());
                i = i + Int64(intLiteral: 1)
            }
            String(storage: RcBox(StringStorage(ptr: newPtr, len: count, cap: count)))
        } else {
            lang.panic("String allocation failed")
        }
    }

    // Properties
    public func byteCount() -> Int64 { self.len() }
    public func capacity() -> Int64 { self.cap() }
    public func isEmpty() -> Bool { self.len() == Int64(intLiteral: 0) }

    // Count code points (not bytes)
    public func count() -> Int64 {
        let myLen = self.len();
        let myPtr = self.ptr();
        var n: Int64 = Int64(intLiteral: 0);
        var i: Int64 = Int64(intLiteral: 0);
        while i < myLen {
            let byte = myPtr.offset(by: i).read();
            // Count leading bytes only (not continuation bytes 10xxxxxx)
            let byteVal: lang.i32 = lang.cast_i8_i32(byte.raw);
            if lang.i32_ne(lang.i32_and(byteVal, 0xC0), 0x80) {
                n = n + Int64(intLiteral: 1)
            }
            i = i + Int64(intLiteral: 1)
        }
        n
    }

    // Byte access
    public func byteAt(index: Int64) -> Optional[UInt8] {
        let myLen = self.len();
        if index >= Int64(intLiteral: 0) and index < myLen {
            .Some(self.ptr().offset(by: index).read())
        } else {
            .None
        }
    }

    public func byteAtUnchecked(index: Int64) -> UInt8 {
        self.ptr().offset(by: index).read()
    }

    // Grow capacity
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
        if result.isSome() {
            let newPtr = result.unwrap().cast[UInt8]();
            let oldStorage = self.storage.getValue();
            // Copy existing bytes
            var i: Int64 = Int64(intLiteral: 0);
            while i < oldStorage.len {
                newPtr.offset(by: i).write(oldStorage.ptr.offset(by: i).read());
                i = i + Int64(intLiteral: 1)
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

    // Append string
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
        var i: Int64 = Int64(intLiteral: 0);
        while i < otherLen {
            s.ptr.offset(by: s.len).write(otherPtr.offset(by: i).read());
            s.len = s.len + Int64(intLiteral: 1);
            i = i + Int64(intLiteral: 1)
        }
        self.storage.setValue(s)
    }

    // Append char
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

    // Append byte (unchecked - caller must ensure valid UTF-8)
    public mutating func appendByte(byte: UInt8) {
        self.grow(self.len() + Int64(intLiteral: 1));
        self.makeUnique();
        var s = self.storage.getValue();
        s.ptr.offset(by: s.len).write(byte);
        s.len = s.len + Int64(intLiteral: 1);
        self.storage.setValue(s)
    }

    // Clear
    public mutating func clear() {
        self.makeUnique();
        var s = self.storage.getValue();
        s.len = Int64(intLiteral: 0);
        self.storage.setValue(s)
    }

    // Substring by byte indices
    public func substringBytes(from start: Int64, to end: Int64) -> String {
        let myLen = self.len();
        if start >= end or start < Int64(intLiteral: 0) or end > myLen {
            return String()
        }
        String.fromBytesUnchecked(self.ptr().offset(by: start), end - start)
    }

    // Search
    public func contains(substring: String) -> Bool {
        self.find(substring).isSome()
    }

    public func find(substring: String) -> Optional[Int64] {
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

    public func startsWith(prefix: String) -> Bool {
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

    public func endsWith(suffix: String) -> Bool {
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

    // Trimming
    public func trim() -> String {
        self.trimStart().trimEnd()
    }

    public func trimStart() -> String {
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

    public func trimEnd() -> String {
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

    // Case conversion (ASCII only)
    public func lowercase() -> String {
        let myLen = self.len();
        let myPtr = self.ptr();
        var result = String(capacity: myLen);
        var i: Int64 = Int64(intLiteral: 0);
        while i < myLen {
            let byte = myPtr.offset(by: i).read();
            let v: lang.i32 = lang.cast_i8_i32(byte.raw);
            // A-Z: 65-90 -> a-z: 97-122
            let isUppercase = lang.i1_and(lang.i32_signed_ge(v, 65), lang.i32_signed_le(v, 90));
            if Bool(boolLiteral: isUppercase) {
                result.appendByte(UInt8(raw: lang.cast_i32_i8(lang.i32_add(v, 32))))
            } else {
                result.appendByte(byte)
            }
            i = i + Int64(intLiteral: 1)
        }
        result
    }

    public func uppercase() -> String {
        let myLen = self.len();
        let myPtr = self.ptr();
        var result = String(capacity: myLen);
        var i: Int64 = Int64(intLiteral: 0);
        while i < myLen {
            let byte = myPtr.offset(by: i).read();
            let v: lang.i32 = lang.cast_i8_i32(byte.raw);
            // a-z: 97-122 -> A-Z: 65-90
            let isLowercase = lang.i1_and(lang.i32_signed_ge(v, 97), lang.i32_signed_le(v, 122));
            if Bool(boolLiteral: isLowercase) {
                result.appendByte(UInt8(raw: lang.cast_i32_i8(lang.i32_sub(v, 32))))
            } else {
                result.appendByte(byte)
            }
            i = i + Int64(intLiteral: 1)
        }
        result
    }

    // Replace
    public func replace(pattern: String, with replacement: String) -> String {
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
                var j: Int64 = Int64(intLiteral: 0);
                while j < patternLen and matches {
                    let a = myPtr.offset(by: i + j).read();
                    let b = patternPtr.offset(by: j).read();
                    if a.equals(b) == false {
                        matches = false
                    }
                    j = j + Int64(intLiteral: 1)
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

    // Split
    public func split(separator: String) -> SplitIterator {
        SplitIterator(
            ptr: self.ptr(),
            length: self.len(),
            sepPtr: separator.ptr(),
            sepLen: separator.len()
        )
    }

    // Iterable
    public func iter() -> StringIterator {
        StringIterator(ptr: self.ptr(), length: self.len())
    }

    // Addable
    public func add(other: String) -> String {
        var result = self.clone();
        result.append(other);
        result
    }

    // Equatable
    public func equals(other: String) -> Bool {
        let myLen = self.len();
        let otherLen = other.len();
        if myLen != otherLen {
            return false
        }
        let myPtr = self.ptr();
        let otherPtr = other.ptr();
        var i: Int64 = Int64(intLiteral: 0);
        var equal: Bool = true;
        while i < myLen and equal {
            let a = myPtr.offset(by: i).read();
            let b = otherPtr.offset(by: i).read();
            if a.equals(b) == false {
                equal = false
            }
            i = i + Int64(intLiteral: 1)
        }
        equal
    }

    // Comparable (lexicographic byte comparison)
    public func compare(other: String) -> Ordering {
        let myLen = self.len();
        let otherLen = other.len();
        var minLen: Int64 = myLen;
        if otherLen < minLen {
            minLen = otherLen
        }

        let myPtr = self.ptr();
        let otherPtr = other.ptr();
        var i: Int64 = Int64(intLiteral: 0);
        while i < minLen {
            let a = myPtr.offset(by: i).read();
            let b = otherPtr.offset(by: i).read();
            let cmp = a.compare(b);
            let eql: Ordering = .Equal;
            if cmp.equals(eql) == false {
                return cmp
            }
            i = i + Int64(intLiteral: 1)
        }

        if myLen < otherLen {
            .Less
        } else if myLen > otherLen {
            .Greater
        } else {
            .Equal
        }
    }

    // Cloneable - shallow clone (COW)
    public func clone() -> String {
        String(storage: self.storage.clone())
    }

    // Formattable
    public func format() -> String {
        self.clone()
    }
}
