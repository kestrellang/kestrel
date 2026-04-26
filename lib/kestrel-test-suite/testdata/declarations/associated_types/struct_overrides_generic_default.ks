// test: diagnostics
// stdlib: false
module Test

struct Array[T] { }
struct LinkedList[T] { }
protocol Collection[T] {
    type Storage = Array[T];
}
struct MyCollection[T]: Collection[T] {
    type Storage = LinkedList[T];
}
