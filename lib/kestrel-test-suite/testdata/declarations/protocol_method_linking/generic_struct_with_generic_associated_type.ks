// test: diagnostics
// stdlib: false
module Test

struct Wrapper[T] {
    var value: T
    init(v: T) { self.value = v }
}
protocol Iterable {
    type Iter
    func iter() -> Iter
}
struct Container[T]: Iterable {
    type Iter = Wrapper[T]
    var data: T
    init(d: T) { self.data = d }
    func iter() -> Wrapper[T] { Wrapper(self.data) }
}
