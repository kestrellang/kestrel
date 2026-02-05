module Test

protocol Iterator {
    type Item
    func next() -> Optional[Item]
}

struct ArrayIterator[T]: Iterator {
    type Item = T
    func next() -> Optional[T] { .None }
}

func test() {
    let iter = ArrayIterator[Int64]()
    let item: Int64 = iter.next().unwrap()  // This should work
}
