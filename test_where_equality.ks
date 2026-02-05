protocol MyProtocol {
    type Item

    func test[T]() where Item = T {
    }
}
