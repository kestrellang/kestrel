// String - UTF-8 encoded string

module std.text

import std.core.(Bool, Equatable, Comparable, Cloneable, Ordering, Addable, ExpressibleByStringLiteral)
import std.num.(Int64, UInt8)
import std.result.(Optional)
import std.memory.(Layout, Pointer, RawPointer, SystemAllocator)
import std.iter.(Iterator, Iterable)
import std.text.(CodePoint, decodeUtf8, encodeUtf8)
import std.ffi.(memcpy)

// StringIterator - iterates over code points
public struct StringIterator: Iterator {
    type Item = CodePoint

    private var ptr: Pointer[UInt8]
    private var length: Int64
    private var index: Int64

    public init(ptr: Pointer[UInt8], length: Int64) {
        self.ptr = ptr;
        self.length = length;
        self.index = Int64(intLiteral: 0);
    }

    public mutating func next() -> Optional[CodePoint] {
        if self.index >= self.length {
            return .None
        }
        // Decode UTF-8 at current position
        let rawPtr: lang.ptr[lang.i8] = lang.cast_ptr[lang.i8](self.ptr.asRaw().raw);
        let result = decodeUtf8(rawPtr, self.length, at: self.index);
        if result.isSome() {
            let decoded = result.unwrap();
            self.index = self.index + decoded.bytesConsumed;
            .Some(decoded.codePoint)
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

    public init(ptr: Pointer[UInt8], length: Int64, sepPtr: Pointer[UInt8], sepLen: Int64) {
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

// String - UTF-8 encoded, dynamically sized string
public struct String: Iterable, Equatable, Comparable, Cloneable, Addable, ExpressibleByStringLiteral {
    type Item = CodePoint
    type Iter = StringIterator
    type Output = String

    private var ptr: Pointer[UInt8]
    private var len: Int64
    private var cap: Int64

    // Create empty string
    public init() {
        self.ptr = Pointer(raw: lang.ptr_null[UInt8]());
        self.len = Int64(intLiteral: 0);
        self.cap = Int64(intLiteral: 0);
    }

    // Create with capacity
    public init(capacity: Int64) {
        if capacity > Int64(intLiteral: 0) {
            let layout = Layout.array[UInt8](capacity);
            var allocator = SystemAllocator();
            let result = allocator.allocate(layout);
            if result.isSome() {
                self.ptr = result.unwrap().cast[UInt8]();
                self.len = Int64(intLiteral: 0);
                self.cap = capacity
            } else {
                lang.panic("String allocation failed")
            }
        } else {
            self.ptr = Pointer(raw: lang.ptr_null[UInt8]());
            self.len = Int64(intLiteral: 0);
            self.cap = Int64(intLiteral: 0)
        }
    }

    // ExpressibleByStringLiteral
    public init(stringLiteral ptr: lang.ptr[lang.i8], length: lang.i64) {
        if length > 0 {
            let byteCount = Int64(intLiteral: length);
            let layout = Layout.array[UInt8](byteCount);
            var allocator = SystemAllocator();
            let result = allocator.allocate(layout);
            if result.isSome() {
                self.ptr = result.unwrap().cast[UInt8]();
                self.len = byteCount;
                self.cap = byteCount;
                // Copy bytes from literal
                let srcPtr: lang.ptr[lang.i8] = ptr;
                let dstPtr: lang.ptr[lang.i8] = lang.cast_ptr[lang.i8](self.ptr.asRaw().raw);
                let _ = memcpy(dstPtr, srcPtr, length);
            } else {
                lang.panic("String allocation failed")
            }
        } else {
            self.ptr = Pointer(raw: lang.ptr_null[UInt8]());
            self.len = Int64(intLiteral: 0);
            self.cap = Int64(intLiteral: 0)
        }
    }

    // Private: create from byte pointer without copying (takes ownership)
    private init(ptr: Pointer[UInt8], len: Int64, cap: Int64) {
        self.ptr = ptr;
        self.len = len;
        self.cap = cap;
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
            String(ptr: newPtr, len: count, cap: count)
        } else {
            lang.panic("String allocation failed")
        }
    }

    deinit {
        if self.cap > Int64(intLiteral: 0) {
            let layout = Layout.array[UInt8](self.cap);
            var allocator = SystemAllocator();
            allocator.deallocate(self.ptr.asRaw(), layout)
        }
    }

    // Properties
    public func byteCount() -> Int64 { self.len }
    public func capacity() -> Int64 { self.cap }
    public func isEmpty() -> Bool { self.len == Int64(intLiteral: 0) }

    // Count code points (not bytes)
    public func count() -> Int64 {
        var n: Int64 = Int64(intLiteral: 0);
        var i: Int64 = Int64(intLiteral: 0);
        while i < self.len {
            let byte = self.ptr.offset(by: i).read();
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
        if index >= Int64(intLiteral: 0) and index < self.len {
            .Some(self.ptr.offset(by: index).read())
        } else {
            .None
        }
    }

    public func byteAtUnchecked(index: Int64) -> UInt8 {
        self.ptr.offset(by: index).read()
    }

    // Subscripts for byte access - commented out due to compiler issue with parameter binding
    // public subscript(safe index: Int64) -> Optional[UInt8] {
    //     get {
    //         if index >= Int64(intLiteral: 0) and index < self.len {
    //             .Some(self.ptr.offset(by: index).read())
    //         } else {
    //             .None
    //         }
    //     }
    // }
    // public subscript(unchecked index: Int64) -> UInt8 {
    //     get { self.ptr.offset(by: index).read() }
    //     set { self.ptr.offset(by: index).write(newValue) }
    // }

    // Grow capacity
    private mutating func grow(minCapacity: Int64) {
        if self.cap >= minCapacity {
            return
        }

        var newCap: Int64 = self.cap;
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
            // Copy existing bytes
            var i: Int64 = Int64(intLiteral: 0);
            while i < self.len {
                newPtr.offset(by: i).write(self.ptr.offset(by: i).read());
                i = i + Int64(intLiteral: 1)
            }
            // Free old buffer
            if self.cap > Int64(intLiteral: 0) {
                let oldLayout = Layout.array[UInt8](self.cap);
                allocator.deallocate(self.ptr.asRaw(), oldLayout)
            }
            self.ptr = newPtr;
            self.cap = newCap
        } else {
            lang.panic("String grow failed")
        }
    }

    // Append string
    public mutating func append(other: String) {
        if other.len == Int64(intLiteral: 0) {
            return
        }
        self.grow(self.len + other.len);
        var i: Int64 = Int64(intLiteral: 0);
        while i < other.len {
            self.ptr.offset(by: self.len).write(other.ptr.offset(by: i).read());
            self.len = self.len + Int64(intLiteral: 1);
            i = i + Int64(intLiteral: 1)
        }
    }

    // Append code point
    public mutating func appendCodePoint(cp: CodePoint) {
        let utf8Len = cp.utf8Length();
        self.grow(self.len + utf8Len);
        // Encode to buffer
        let rawPtr: lang.ptr[lang.i8] = lang.cast_ptr[lang.i8](self.ptr.asRaw().raw);
        let written = encodeUtf8(cp, rawPtr, at: self.len);
        self.len = self.len + written
    }

    // Append byte (unchecked - caller must ensure valid UTF-8)
    public mutating func appendByte(byte: UInt8) {
        self.grow(self.len + Int64(intLiteral: 1));
        self.ptr.offset(by: self.len).write(byte);
        self.len = self.len + Int64(intLiteral: 1)
    }

    // Clear
    public mutating func clear() {
        self.len = Int64(intLiteral: 0)
    }

    // Substring by byte indices
    public func substringBytes(from start: Int64, to end: Int64) -> String {
        if start >= end or start < Int64(intLiteral: 0) or end > self.len {
            return String()
        }
        String.fromBytesUnchecked(self.ptr.offset(by: start), end - start)
    }

    // Search
    public func contains(substring: String) -> Bool {
        self.find(substring).isSome()
    }

    public func find(substring: String) -> Optional[Int64] {
        if substring.len == Int64(intLiteral: 0) {
            return .Some(Int64(intLiteral: 0))
        }
        if substring.len > self.len {
            return .None
        }

        var i: Int64 = Int64(intLiteral: 0);
        let lastStart = self.len - substring.len;
        while i <= lastStart {
            var matches: Bool = true;
            var j: Int64 = Int64(intLiteral: 0);
            while j < substring.len and matches {
                let a = self.ptr.offset(by: i + j).read();
                let b = substring.ptr.offset(by: j).read();
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
        if prefix.len > self.len {
            return false
        }
        var i: Int64 = Int64(intLiteral: 0);
        var matches: Bool = true;
        while i < prefix.len and matches {
            let a = self.ptr.offset(by: i).read();
            let b = prefix.ptr.offset(by: i).read();
            if a.equals(b) == false {
                matches = false
            }
            i = i + Int64(intLiteral: 1)
        }
        matches
    }

    public func endsWith(suffix: String) -> Bool {
        if suffix.len > self.len {
            return false
        }
        let offset = self.len - suffix.len;
        var i: Int64 = Int64(intLiteral: 0);
        var matches: Bool = true;
        while i < suffix.len and matches {
            let a = self.ptr.offset(by: offset + i).read();
            let b = suffix.ptr.offset(by: i).read();
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
        var start: Int64 = Int64(intLiteral: 0);
        while start < self.len {
            let byte = self.ptr.offset(by: start).read();
            let v: lang.i32 = lang.cast_i8_i32(byte.raw);
            // space=32, tab=9, newline=10, carriage return=13
            if v == 32 or v == 9 or v == 10 or v == 13 {
                start = start + Int64(intLiteral: 1)
            } else {
                // break out
                start = self.len + Int64(intLiteral: 1)
            }
        }
        if start > self.len {
            start = start - Int64(intLiteral: 1) - self.len + self.len
        }
        // Reconstruct start as the first non-whitespace position
        var realStart: Int64 = Int64(intLiteral: 0);
        var done: Bool = false;
        while realStart < self.len and done == false {
            let byte = self.ptr.offset(by: realStart).read();
            let v: lang.i32 = lang.cast_i8_i32(byte.raw);
            if v == 32 or v == 9 or v == 10 or v == 13 {
                realStart = realStart + Int64(intLiteral: 1)
            } else {
                done = true
            }
        }
        self.substringBytes(from: realStart, to: self.len)
    }

    public func trimEnd() -> String {
        var endPos: Int64 = self.len;
        var done: Bool = false;
        while endPos > Int64(intLiteral: 0) and done == false {
            let idx = endPos - Int64(intLiteral: 1);
            let byte = self.ptr.offset(by: idx).read();
            let v: lang.i32 = lang.cast_i8_i32(byte.raw);
            if v == 32 or v == 9 or v == 10 or v == 13 {
                endPos = endPos - Int64(intLiteral: 1)
            } else {
                done = true
            }
        }
        self.substringBytes(from: Int64(intLiteral: 0), to: endPos)
    }

    // Case conversion (ASCII only)
    public func lowercase() -> String {
        var result = String(capacity: self.len);
        var i: Int64 = Int64(intLiteral: 0);
        while i < self.len {
            let byte = self.ptr.offset(by: i).read();
            let v: lang.i32 = lang.cast_i8_i32(byte.raw);
            // A-Z: 65-90 -> a-z: 97-122
            if v >= 65 and v <= 90 {
                result.appendByte(UInt8(raw: lang.cast_i32_i8(v + 32)))
            } else {
                result.appendByte(byte)
            }
            i = i + Int64(intLiteral: 1)
        }
        result
    }

    public func uppercase() -> String {
        var result = String(capacity: self.len);
        var i: Int64 = Int64(intLiteral: 0);
        while i < self.len {
            let byte = self.ptr.offset(by: i).read();
            let v: lang.i32 = lang.cast_i8_i32(byte.raw);
            // a-z: 97-122 -> A-Z: 65-90
            if v >= 97 and v <= 122 {
                result.appendByte(UInt8(raw: lang.cast_i32_i8(v - 32)))
            } else {
                result.appendByte(byte)
            }
            i = i + Int64(intLiteral: 1)
        }
        result
    }

    // Replace
    public func replace(pattern: String, with replacement: String) -> String {
        if pattern.len == Int64(intLiteral: 0) {
            return self.clone()
        }

        var result = String();
        var i: Int64 = Int64(intLiteral: 0);

        while i < self.len {
            // Check for pattern match
            var matches: Bool = true;
            if i + pattern.len <= self.len {
                var j: Int64 = Int64(intLiteral: 0);
                while j < pattern.len and matches {
                    let a = self.ptr.offset(by: i + j).read();
                    let b = pattern.ptr.offset(by: j).read();
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
                i = i + pattern.len
            } else {
                result.appendByte(self.ptr.offset(by: i).read());
                i = i + Int64(intLiteral: 1)
            }
        }
        result
    }

    // Split
    public func split(separator: String) -> SplitIterator {
        SplitIterator(
            ptr: self.ptr,
            length: self.len,
            sepPtr: separator.ptr,
            sepLen: separator.len
        )
    }

    // Iterable
    public func iter() -> StringIterator {
        StringIterator(ptr: self.ptr, length: self.len)
    }

    // Addable
    public func add(other: String) -> String {
        var result = self.clone();
        result.append(other);
        result
    }

    // Equatable
    public func equals(other: String) -> Bool {
        if self.len != other.len {
            return false
        }
        var i: Int64 = Int64(intLiteral: 0);
        var equal: Bool = true;
        while i < self.len and equal {
            let a = self.ptr.offset(by: i).read();
            let b = other.ptr.offset(by: i).read();
            if a.equals(b) == false {
                equal = false
            }
            i = i + Int64(intLiteral: 1)
        }
        equal
    }

    // Comparable (lexicographic byte comparison)
    public func compare(other: String) -> Ordering {
        var minLen: Int64 = self.len;
        if other.len < minLen {
            minLen = other.len
        }

        var i: Int64 = Int64(intLiteral: 0);
        while i < minLen {
            let a = self.ptr.offset(by: i).read();
            let b = other.ptr.offset(by: i).read();
            let cmp = a.compare(b);
            let eql: Ordering = .Equal;
            if cmp.equals(eql) == false {
                return cmp
            }
            i = i + Int64(intLiteral: 1)
        }

        if self.len < other.len {
            .Less
        } else if self.len > other.len {
            .Greater
        } else {
            .Equal
        }
    }

    // Hashable - TODO: implement when Hasher has writeU8
    // public mutating func hash(into hasher: Hasher) {
    //     var i: Int64 = Int64(intLiteral: 0);
    //     while i < self.len {
    //         let byte = self.ptr.offset(by: i).read();
    //         hasher.writeU8(byte);
    //         i = i + Int64(intLiteral: 1)
    //     }
    // }

    // Cloneable
    public func clone() -> String {
        if self.len == Int64(intLiteral: 0) {
            return String()
        }
        String.fromBytesUnchecked(self.ptr, self.len)
    }
}
