// test: diagnostics
// stdlib: false

module Test

protocol Defaultable {
    static func default_() -> Self
}

struct Container[T] where T: Defaultable {
    var item: T

    init(item: T) {
        self.item = item
    }

    init() {
        self.init(T.default_())
    }
}
