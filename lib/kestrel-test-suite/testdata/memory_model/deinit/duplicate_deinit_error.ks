// test: diagnostics
// stdlib: false

module Test

struct Handle: not Copyable {
    var fd: lang.i64

    deinit {}

    deinit {} // ERROR: already has a deinit
}
