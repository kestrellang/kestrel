// test: diagnostics
// stdlib: false

module Test
import Prelude

struct Resource: not Copyable {
    var id: lang.i64

    init(id: lang.i64) {
        self.id = id
    }

    deinit {}
}
