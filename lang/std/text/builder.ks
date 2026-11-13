// StringBuilder - write-only buffer for efficient string construction

module std.text

import std.core.(Bool, Cloneable, Defaultable, fatalError)
import std.numeric.(Int64, UInt8)
import std.memory.(Allocator, GlobalAllocator, Layout, Pointer, RawPointer, RcBox, CowBox)
import std.text.(Char, encodeUtf8, Formatter, String, StringSlice, StringStorage, _textAlloc, _textDealloc, _memcpyBytes)

/// Write-only buffer for efficient string construction. No COW, no
/// RcBox, no `isUnique` checks — every append writes directly.
///
/// `build()` transfers ownership of the buffer into a new `String`
/// without copying. The builder resets to empty and can be reused.
///
/// # Examples
///
/// ```
/// var b = StringBuilder();
/// b.append("hello");
/// b.append(char: ' ');
/// b.append("world");
/// let s = b.build();   // "hello world", zero-copy
/// ```
///
/// # Representation
///
/// `(ptr: Pointer[UInt8], len: Int64, cap: Int64, allocator: A)`.
///
/// # Memory Model
///
/// Owns its buffer directly. `build()` donates the buffer to a
/// `String`; the builder is left empty. `deinit` frees the buffer
/// if `build()` was never called.
public struct StringBuilder[A = GlobalAllocator]: Cloneable, Formatter where A: Allocator {
    private var ptr: Pointer[UInt8]
    private var len: Int64
    private var cap: Int64
    // The buffer's allocator; the built `String` takes it along.
    private var allocator: A

    /// @name Empty With Allocator
    /// Creates an empty builder whose buffer will come from `allocator`.
    public init(allocator allocator: A) {
        self.ptr = Pointer[UInt8].nullPointer();
        self.len = 0;
        self.cap = 0;
        self.allocator = allocator;
    }

    /// @name With Capacity and Allocator
    /// Creates an empty builder with at least `capacity` bytes preallocated
    /// from `allocator`.
    public init(capacity capacity: Int64, allocator allocator: A) {
        self.init(allocator: allocator);
        if capacity > 0 {
            self.ptr = _textAlloc(Layout.array[UInt8](capacity), from: allocator);
            self.cap = capacity
        }
    }

    // -- Growth --------------------------------------------------------------

    private mutating func grow(minCapacity: Int64) {
        if self.cap >= minCapacity { return }
        var newCap = self.cap;
        if newCap == 0 { newCap = 16 }
        while newCap < minCapacity {
            newCap = newCap * 2
        }
        let newPtr = _textAlloc(Layout.array[UInt8](newCap), from: self.allocator);
        if self.len > 0 {
            _memcpyBytes(dst: newPtr, src: self.ptr, n: self.len)
        }
        if self.cap > 0 {
            _textDealloc(self.ptr, Layout.array[UInt8](self.cap), to: self.allocator)
        }
        self.ptr = newPtr;
        self.cap = newCap
    }

    // -- Appending -----------------------------------------------------------

    /// Appends the UTF-8 bytes of `other` to this builder: a `String`
    /// lends its slice.
    public mutating func append(other: &StringSlice) {
        let otherLen = other._len;
        if otherLen == 0 { return }
        self.grow(self.len + otherLen);
        let srcPtr = other._rawPtr();
        _memcpyBytes(dst: self.ptr.offset(by: self.len), src: srcPtr, n: otherLen);
        self.len = self.len + otherLen
    }

    /// Appends a single code point, encoding it as UTF-8.
    public mutating func append(char c: Char) {
        let utf8Len = c.utf8Length();
        self.grow(self.len + utf8Len);
        let rawPtr: lang.ptr[lang.i8] = lang.cast_ptr[_, lang.i8](self.ptr.asRaw().raw);
        let written = encodeUtf8(c, rawPtr, at: self.len);
        self.len = self.len + written
    }

    /// Appends a raw byte. Caller must ensure UTF-8 validity.
    internal mutating func appendByte(byte: UInt8) {
        self.grow(self.len + 1);
        unsafe self.ptr.offset(by: self.len).write(byte);
        self.len = self.len + 1
    }

    /// Appends `count` bytes from `ptr`. Caller must ensure UTF-8 validity.
    internal mutating func appendBytes(ptr ptr: Pointer[UInt8], count count: Int64) {
        if count <= 0 { return }
        self.grow(self.len + count);
        _memcpyBytes(dst: self.ptr.offset(by: self.len), src: ptr, n: count);
        self.len = self.len + count
    }

    // -- Build ---------------------------------------------------------------

    /// Transfers the buffer into a new `String` without copying.
    /// The builder resets to empty and can be reused.
    public mutating func build() -> String[A] {
        if self.len == 0 {
            return String[A](allocator: self.allocator)
        }
        // Move the buffer into a fresh String. Disarm `self` FIRST so its deinit
        // can't free the buffer the String now owns, then MOVE `storage` into the
        // CowBox as its LAST use. `storage` must NOT be read afterward: a later
        // use forces a `copy_value`, and because `StringStorage` is `Cloneable`
        // that expands to a `clone()` — allocating a *fresh* buffer for the String
        // and orphaning this builder's original buffer (one StringBuilder buffer
        // leaked per `formatted()` / string-interpolation call).
        let storage = StringStorage(ptr: self.ptr, len: self.len, cap: self.cap, allocator: self.allocator);
        self.ptr = Pointer[UInt8].nullPointer();
        self.len = 0;
        self.cap = 0;
        String(storage: CowBox(storage))
    }

    /// Resets length to zero, keeping the allocated buffer for reuse.
    public mutating func clear() {
        self.len = 0
    }

    // -- Queries -------------------------------------------------------------

    /// Number of bytes written so far.
    public var count: Int64 { self.len }

    /// True when nothing has been written.
    public var isEmpty: Bool { self.len == 0 }

    // -- Clone ---------------------------------------------------------------

    /// Returns a copy with its own buffer.
    public func clone() -> StringBuilder[A] {
        var copy = StringBuilder[A](capacity: self.cap, allocator: self.allocator);
        if self.len > 0 {
            _memcpyBytes(dst: copy.ptr, src: self.ptr, n: self.len);
            copy.len = self.len
        }
        copy
    }

    // -- Cleanup -------------------------------------------------------------

    deinit {
        if self.cap > 0 {
            _textDealloc(self.ptr, Layout.array[UInt8](self.cap), to: self.allocator)
        }
    }
}

extend StringBuilder[A] where A: Defaultable {
    /// @name Empty
    /// Creates an empty builder with no allocation.
    public init() {
        self.init(allocator: A())
    }

    /// @name With Capacity
    /// Creates an empty builder with at least `capacity` bytes preallocated.
    public init(capacity capacity: Int64) {
        self.init(capacity: capacity, allocator: A())
    }
}
