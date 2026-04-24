// test: diagnostics
// stdlib: false

module Test
import Prelude

struct Handle: not Copyable {
    var fd: lang.i64

    deinit {}

    deinit {} // ERROR: already has a deinit
}
