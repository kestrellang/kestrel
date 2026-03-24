// test: diagnostics
// stdlib: false

module Test
import Prelude

struct Connection: not Copyable {
    var host: lang.str
    var port: lang.i64
    var connected: lang.i1

    deinit {}
}
