// test: diagnostics
// stdlib: false

module Test

@builtin(.Copyable)
protocol Copyable {}

struct Connection: not Copyable {
    var host: lang.str
    var port: lang.i64
    var connected: lang.i1

    deinit {}
}
