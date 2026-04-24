// test: diagnostics
// stdlib: false

module Test

public struct Pointer[T] {
    var raw: lang.ptr[T]

    public init(raw: lang.ptr[T]) {
        self.raw = raw;
    }
}

public func testCastPtr[T]() -> Pointer[T] {
    Pointer(raw: lang.cast_ptr[_, T](lang.ptr_null()))
}
