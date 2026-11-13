// test: diagnostics
// stdlib: false

module Test

@builtin(.Copyable)
protocol Copyable {}

struct Resource: not Copyable {
    var id: lang.i64

    init(id: lang.i64) {
        self.id = id
    }

    deinit {}
}
