// test: diagnostics
// stdlib: false

module Test

protocol Hashable { func hash() -> lang.i64 }

func getHash[T](value: T) -> lang.i64 where T: Hashable {
    value.hash()
}
