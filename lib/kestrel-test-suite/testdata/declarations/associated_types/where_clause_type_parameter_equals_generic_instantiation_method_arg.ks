// test: diagnostics
// stdlib: false
module Test

struct Array[T] {
    func append(element: T) { }
}
func pushOne[E, V](arr: V, elem: E) where V = Array[E] {
    arr.append(elem);
}
