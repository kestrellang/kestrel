// test: diagnostics
// stdlib: false

module Test

protocol HasCount {
    var count: lang.i64 { get }
}

struct Collection: HasCount {
    var items: lang.i64

    var count: lang.i64 {
        self.items
    }
}
