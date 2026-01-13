// Pointer types

module std.memory

import std.ffi.(FFISafe)
import std.core.(Equatable, UInt, Int, Bool)
import std.result.(Optional)
import std.iter.(Iterator, Iterable)

// RawPointer - untyped pointer
// RawPointer is always FFI-safe as it's just an opaque pointer
public struct RawPointer: Equatable, FFISafe {
    private var raw: lang.ptr[lang.u8]

    public init(raw: lang.ptr[lang.u8]) {
        self.raw = raw;
    }

    public init(address: UInt) {
        self.raw = lang.ptr_from_address(address)
    }

    public static func nilPointer() -> RawPointer {
        RawPointer(raw: lang.ptr_null())
    }

    public var address: UInt {
        lang.ptr_to_address(self.raw)
    }

    public var isNull: Bool {
        lang.ptr_is_null(self.raw)
    }

    public func cast[T]() -> Pointer[T] {
        Pointer(raw: lang.cast_ptr[T](self.raw))
    }

    public func offset(by bytes: Int) -> RawPointer {
        RawPointer(raw: lang.ptr_offset(self.raw, bytes))
    }

    public func equals(other: RawPointer) -> Bool {
        self.address == other.address
    }
}

// Pointer[T] - typed pointer to a single element
public struct Pointer[T]: Equatable {
    private var raw: lang.ptr[T]

    public init(raw: lang.ptr[T]) {
        self.raw = raw;
    }

    public init(to value: T) {
        self.raw = lang.ptr_to(value)
    }

    public static func nilPointer() -> Pointer[T] {
        Pointer(raw: lang.cast_ptr[T](lang.ptr_null()))
    }

    public var pointee: T {
        get { lang.ptr_read(self.raw) }
        set { lang.ptr_write(self.raw, newValue) }
    }

    public var address: UInt {
        lang.ptr_to_address(lang.cast_ptr[lang.u8](self.raw))
    }

    public var isNull: Bool {
        lang.ptr_is_null(lang.cast_ptr[lang.u8](self.raw))
    }

    public func read() -> T {
        lang.ptr_read(self.raw)
    }

    public func write(value: T) {
        lang.ptr_write(self.raw, value)
    }

    public func offset(by n: Int) -> Pointer[T] {
        Pointer(raw: lang.cast_ptr[T](lang.ptr_offset(self.raw, n * lang.sizeof[T]())))
    }

    public func asRaw() -> RawPointer {
        RawPointer(raw: lang.cast_ptr[lang.u8](self.raw))
    }

    public func equals(other: Pointer[T]) -> Bool {
        self.address == other.address
    }
}

// Pointer[T] is FFI-safe when T is FFI-safe
extend Pointer: FFISafe where T: FFISafe {}

// Slice[T] - view into contiguous memory
public struct Slice[T]: Iterable, Equatable {
    type Item = T
    type Iter = SliceIterator[T]

    private var ptr: Pointer[T]
    private var len: Int

    public init(pointer: Pointer[T], count: Int) {
        self.ptr = pointer;
        self.len = count;
    }

    public var count: Int { self.len }

    public var isEmpty: Bool { self.len == 0 }

    public var pointer: Pointer[T] { self.ptr }

    // Safe access
    //public subscript(safe index: Int) -> Optional[T] {
    //    get {
    //        if index >= 0 and index < self.len {
    //            .Some(self.ptr.offset(by: index).read())
    //        } else {
    //            .None
    //        }
    //    }
    //}

    // Unchecked access
    //public subscript(unchecked index: Int) -> T {
    //    get { self.ptr.offset(by: index).read() }
    //    set { self.ptr.offset(by: index).write(newValue) }
    //}

    // Slicing
    public func slice(from start: Int, to end: Int) -> Optional[Slice[T]] {
        if start >= 0 and end <= self.len and start <= end {
            .Some(Slice(pointer: self.ptr.offset(by: start), count: end - start))
        } else {
            .None
        }
    }

    // Iteration
    public func iter() -> SliceIterator[T] {
        SliceIterator(ptr: self.ptr, remaining: self.len)
    }

    // First and last
    public func first() -> Optional[T] {
        if self.len > 0 {
            .Some(self.ptr.read())
        } else {
            .None
        }
    }

    public func last() -> Optional[T] {
        if self.len > 0 {
            .Some(self.ptr.offset(by: self.len - 1).read())
        } else {
            .None
        }
    }

    public func equals(other: Slice[T]) -> Bool {
        if self.len != other.len {
            return false
        }
        /* for i in 0..<self.len {
            if self(unchecked: i) != other(unchecked: i) {
                return false
            }
        } */
        true
    }
}

public struct SliceIterator[T]: Iterator {
    type Item = T

    private var ptr: Pointer[T]
    private var remaining: Int

    public init(ptr: Pointer[T], remaining: Int) {
        self.ptr = ptr;
        self.remaining = remaining;
    }

    public mutating func next() -> Optional[T] {
        if self.remaining > 0 {
            let value = self.ptr.read();
            self.ptr = self.ptr.offset(by: 1);
            self.remaining = self.remaining - 1;
            .Some(value)
        } else {
            .None
        }
    }
}
