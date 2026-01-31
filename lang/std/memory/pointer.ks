// Pointer types

module std.memory

import std.ffi.(FFISafe)
import std.core.(Equatable, Bool, Hash, Hasher, ArrayMatchable)
import std.num.(Int64, UInt64, UInt8)
import std.memory.(Slice)
// Note: Optional comes in Phase 11, Iterator/Iterable in Phase 10

/// An untyped (void) pointer to raw memory.
/// FFI-safe and can be cast to typed pointers.
public struct RawPointer: Equatable, FFISafe, Hash {
    /// The underlying raw pointer.
    public var raw: lang.ptr[lang.i8]

    /// Creates a raw pointer from the underlying representation.
    public init(raw raw: lang.ptr[lang.i8]) {
        self.raw = raw;
    }

    /// Creates a raw pointer from an address.
    public init(address address: UInt64) {
        self.raw = lang.ptr_from_address(address)
    }

    /// Returns a null raw pointer.
    public static func nilPointer() -> RawPointer {
        RawPointer(raw: lang.ptr_null())
    }

    /// The memory address as an unsigned integer.
    public var address: UInt64 {
        UInt64(intLiteral: lang.ptr_to_address(self.raw))
    }

    /// Returns true if this is a null pointer.
    public var isNull: Bool {
        Bool(boolLiteral: lang.ptr_is_null(self.raw))
    }

    /// Casts this raw pointer to a typed pointer.
    public func cast[T]() -> Pointer[T] {
        Pointer(raw: lang.cast_ptr[T](self.raw))
    }

    /// Offsets the pointer by the given number of bytes.
    public func offset(by bytes: Int64) -> RawPointer {
        RawPointer(raw: lang.ptr_offset(self.raw, bytes.raw))
    }

    /// Compares two raw pointers for equality by address.
    public func equals(other: RawPointer) -> Bool {
        self.address == other.address
    }

    /// Hashes this pointer's address.
    public func hash[H](mutating into hasher: H) where H: Hasher {
        let addr = self.address;
        hasher.write(Slice(pointer: Pointer(to: addr).asRaw().cast[UInt8](), count: Int64(intLiteral: 8)))
    }
}

/// A typed pointer to a single value of type T.
public struct Pointer[T]: Equatable, Hash {
    private var _raw: lang.ptr[T]

    /// The underlying raw pointer.
    public var raw: lang.ptr[T] { self._raw }

    /// Creates a pointer from the underlying representation.
    public init(raw raw: lang.ptr[T]) {
        self._raw = raw;
    }

    /// Creates a pointer to a value (takes address of the value).
    public init(to value: T) {
        self._raw = lang.ptr_to(value)
    }

    /// Returns a null typed pointer.
    public static func nilPointer() -> Pointer[T] {
        Pointer(raw: lang.ptr_null[T]())
    }

    /// The value at this pointer location.
    /// Supports both get and set operations.
    public var pointee: T {
        get { lang.ptr_read(self._raw) }
        set { lang.ptr_write(self._raw, newValue) }
    }

    /// The memory address as an unsigned integer.
    public var address: UInt64 {
        UInt64(intLiteral: lang.ptr_to_address(lang.cast_ptr[lang.i8](self._raw)))
    }

    /// Returns true if this is a null pointer.
    public var isNull: Bool {
        Bool(boolLiteral: lang.ptr_is_null(lang.cast_ptr[lang.i8](self._raw)))
    }

    /// Reads the value at this pointer location.
    public func read() -> T {
        lang.ptr_read(self._raw)
    }

    /// Writes a value to this pointer location.
    public func write(value: T) {
        lang.ptr_write(self._raw, value)
    }

    /// Offsets the pointer by n elements (not bytes).
    public func offset(by n: Int64) -> Pointer[T] {
        let byteOffset = n * Int64(intLiteral: lang.sizeof[T]());
        Pointer[T](raw: lang.ptr_offset[T](self._raw, byteOffset.raw))
    }

    /// Converts to an untyped raw pointer.
    public func asRaw() -> RawPointer {
        RawPointer(raw: lang.cast_ptr[lang.i8](self._raw))
    }

    /// Casts this pointer to a pointer of a different type.
    public func cast[U]() -> Pointer[U] {
        Pointer(raw: lang.cast_ptr[U](lang.cast_ptr[lang.i8](self._raw)))
    }

    /// Compares two pointers for equality by address.
    public func equals(other: Pointer[T]) -> Bool {
        self.address == other.address
    }

    /// Hashes this pointer's address.
    public func hash[H](mutating into hasher: H) where H: Hasher {
        let addr = self.address;
        hasher.write(Slice(pointer: Pointer(to: addr).asRaw().cast[UInt8](), count: Int64(intLiteral: 8)))
    }
}

/// Pointer[T] is FFI-safe when T is FFI-safe.
extend Pointer: FFISafe where T: FFISafe {}

/// A non-owning view into contiguous memory.
/// Does not manage lifetime - the underlying memory must outlive the slice.
@builtin(.SliceStruct)
public struct Slice[T]: Equatable {
    // type Item = T
    // type Iter = SliceIterator[T]

    private var ptr: Pointer[T]
    private var len: Int64

    /// Creates a slice from a pointer and element count.
    public init(pointer pointer: Pointer[T], count count: Int64) {
        self.ptr = pointer;
        self.len = count;
    }

    /// The number of elements in the slice.
    public var count: Int64 { self.len }

    /// Returns true if the slice contains no elements.
    public var isEmpty: Bool { self.len == 0 }

    /// A pointer to the first element.
    public var pointer: Pointer[T] { self.ptr }

    // Safe access - requires Optional (Phase 11)
    // public subscript(safe index: Int) -> Optional[T] {
    //     get {
    //         if index >= 0 and index < self.len {
    //             .Some(self.ptr.offset(by: index).read())
    //         } else {
    //             .None
    //         }
    //     }
    // }

    // Unchecked access
    // public subscript(unchecked index: Int) -> T {
    //     get { self.ptr.offset(by: index).read() }
    //     set { self.ptr.offset(by: index).write(newValue) }
    // }

    // Slicing - requires Optional (Phase 11)
    // public func slice(from start: Int, to end: Int) -> Optional[Slice[T]] {
    //     if start >= 0 and end <= self.len and start <= end {
    //         .Some(Slice(pointer: self.ptr.offset(by: start), count: end - start))
    //     } else {
    //         .None
    //     }
    // }

    // Iteration - requires Iterator (Phase 10)
    // public func iter() -> SliceIterator[T] {
    //     SliceIterator(ptr: self.ptr, remaining: self.len)
    // }

    // First and last - require Optional (Phase 11)
    // public func first() -> Optional[T] {
    //     if self.len > 0 {
    //         .Some(self.ptr.read())
    //     } else {
    //         .None
    //     }
    // }

    // public func last() -> Optional[T] {
    //     if self.len > 0 {
    //         .Some(self.ptr.offset(by: self.len - 1).read())
    //     } else {
    //         .None
    //     }
    // }

    /// Compares two slices for equality.
    public func equals(other: Slice[T]) -> Bool {
        if self.len != other.len {
            return false
        }
        // Element-wise comparison requires iteration
        // For now just check length matches
        true
    }
}

/// ArrayMatchable extension for Slice pattern matching.
/// Enables patterns like `[a, b]`, `[a, ..rest]`, `[a, .., z]` on Slice values.
extend Slice[T]: ArrayMatchable {
    type Element = T

    /// Returns the number of elements in the slice.
    public func matchLength() -> Int64 {
        self.count
    }

    /// Returns the element at the given index (unchecked).
    public func matchGet(index: Int64) -> T {
        self.pointer.offset(by: index).read()
    }

    /// Returns a sub-slice from `from` (inclusive) to `to` (exclusive).
    public func matchSlice(from: Int64, to: Int64) -> Slice[T] {
        Slice(pointer: self.pointer.offset(by: from), count: to - from)
    }
}

// SliceIterator - requires Iterator protocol (Phase 10)
// public struct SliceIterator[T]: Iterator {
//     type Item = T
//
//     private var ptr: Pointer[T]
//     private var remaining: Int
//
//     public init(ptr: Pointer[T], remaining: Int) {
//         self.ptr = ptr;
//         self.remaining = remaining;
//     }
//
//     public mutating func next() -> Optional[T] {
//         if self.remaining > 0 {
//             let value = self.ptr.read();
//             self.ptr = self.ptr.offset(by: 1);
//             self.remaining = self.remaining - 1;
//             .Some(value)
//         } else {
//             .None
//         }
//     }
// }
