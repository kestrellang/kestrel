// Pointer types

module std.memory

import std.ffi.(FFISafe)
import std.core.(Equatable, Bool)
import std.num.(Int64, UInt64)
// Note: Optional comes in Phase 11, Iterator/Iterable in Phase 10

// RawPointer - untyped pointer
// RawPointer is always FFI-safe as it's just an opaque pointer
public struct RawPointer: Equatable, FFISafe {
    public var raw: lang.ptr[lang.i8]

    public init(raw raw: lang.ptr[lang.i8]) {
        self.raw = raw;
    }

    public init(address address: UInt64) {
        self.raw = lang.ptr_from_address(address)
    }

    public static func nilPointer() -> RawPointer {
        RawPointer(raw: lang.ptr_null())
    }

    public var address: UInt64 {
        UInt64(intLiteral: lang.ptr_to_address(self.raw))
    }

    public var isNull: Bool {
        Bool(boolLiteral: lang.ptr_is_null(self.raw))
    }

    public func cast[T]() -> Pointer[T] {
        Pointer(raw: lang.cast_ptr[T](self.raw))
    }

    public func offset(by bytes: Int64) -> RawPointer {
        RawPointer(raw: lang.ptr_offset(self.raw, bytes.raw))
    }

    public func equals(other: RawPointer) -> Bool {
        self.address == other.address
    }
}

// Pointer[T] - typed pointer to a single element
public struct Pointer[T]: Equatable {
    private var _raw: lang.ptr[T]

    // Public getter for FFI interop
    public var raw: lang.ptr[T] { self._raw }

    public init(raw raw: lang.ptr[T]) {
        self._raw = raw;
    }

    public init(to value: T) {
        self._raw = lang.ptr_to(value)
    }

    public static func nilPointer() -> Pointer[T] {
        Pointer(raw: lang.ptr_null[T]())
    }

    public var pointee: T {
        get { lang.ptr_read(self._raw) }
        set { lang.ptr_write(self._raw, newValue) }
    }

    public var address: UInt64 {
        UInt64(intLiteral: lang.ptr_to_address(lang.cast_ptr[lang.i8](self._raw)))
    }

    public var isNull: Bool {
        Bool(boolLiteral: lang.ptr_is_null(lang.cast_ptr[lang.i8](self._raw)))
    }

    public func read() -> T {
        lang.ptr_read(self._raw)
    }

    public func write(value: T) {
        lang.ptr_write(self._raw, value)
    }

    public func offset(by n: Int64) -> Pointer[T] {
        let byteOffset = n * Int64(intLiteral: lang.sizeof[T]());
        Pointer[T](raw: lang.ptr_offset[T](self._raw, byteOffset.raw))
    }

    public func asRaw() -> RawPointer {
        RawPointer(raw: lang.cast_ptr[lang.i8](self._raw))
    }

    public func equals(other: Pointer[T]) -> Bool {
        self.address == other.address
    }
}

// Pointer[T] is FFI-safe when T is FFI-safe
extend Pointer: FFISafe where T: FFISafe {}

// Slice[T] - view into contiguous memory
// Note: Iterable conformance requires Iterator (Phase 10)
public struct Slice[T]: Equatable {
    // type Item = T
    // type Iter = SliceIterator[T]

    private var ptr: Pointer[T]
    private var len: Int64

    public init(pointer pointer: Pointer[T], count count: Int64) {
        self.ptr = pointer;
        self.len = count;
    }

    public var count: Int64 { self.len }

    public var isEmpty: Bool { self.len == 0 }

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

    public func equals(other: Slice[T]) -> Bool {
        if self.len != other.len {
            return false
        }
        // Element-wise comparison requires iteration
        // For now just check length matches
        true
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
