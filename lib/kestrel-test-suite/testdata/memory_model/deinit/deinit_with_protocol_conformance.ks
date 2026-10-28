// test: diagnostics
// stdlib: false

module Test

protocol Resource {}

struct Handle: Resource, not Copyable {
    var fd: lang.i64

    deinit {}
}
