protocol Iterator {
    type Item

    public func compactMap[T]() -> Self where Item = Optional[T] {
        return self
    }
}
