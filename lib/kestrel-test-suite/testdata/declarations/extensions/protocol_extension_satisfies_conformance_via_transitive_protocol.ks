// test: diagnostics
// stdlib: false
module Test

protocol Equatable {
    func isEqual(to other: Self) -> lang.i1
}

// `-> Self`, not `-> Slice[T]`: a bare protocol is not a value type (E144).
protocol Slice[T] {
    func asSlice() -> Self
}

// `isEqual` provided by the protocol extension on Slice.
// `MyArr[T]: Equatable where T: Equatable` should be satisfied by this
// extension's `isEqual` (Self = MyArr[T] when applied to MyArr).
extend Slice[T] where T: Equatable {
    public func isEqual(to other: Self) -> lang.i1 {
        return self.asSlice().isEqual(to: other.asSlice());
    }
}

struct MyArr[T]: Slice[T] {
    func asSlice() -> MyArr[T] { self }
}

extend MyArr[T]: Equatable where T: Equatable { }
