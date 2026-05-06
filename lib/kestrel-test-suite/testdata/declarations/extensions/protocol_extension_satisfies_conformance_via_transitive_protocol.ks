// test: diagnostics
// stdlib: false
module Test

protocol Equatable {
    func isEqual(to other: Self) -> lang.i1
}

protocol Slice[T] {
    func asSlice() -> Slice[T]
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
    func asSlice() -> Slice[T] { self }
}

extend MyArr[T]: Equatable where T: Equatable { }
