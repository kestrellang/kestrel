// test: diagnostics
// stdlib: false

module Test
import Prelude

protocol Resource {}

struct Handle: Resource, not Copyable {
    var fd: lang.i64

    deinit {}
}
