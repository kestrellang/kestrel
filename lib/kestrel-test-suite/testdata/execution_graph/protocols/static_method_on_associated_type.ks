// test: diagnostics
// stdlib: false

module Test

protocol Factory {
    static func create() -> Self
}

protocol Container {
    type Item: Factory;
}

func makeItem[T]() -> T.Item where T: Container {
    return T.Item.create()
}
