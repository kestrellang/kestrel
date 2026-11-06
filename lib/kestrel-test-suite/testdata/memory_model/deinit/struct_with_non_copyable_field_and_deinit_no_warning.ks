// test: diagnostics
// stdlib: false

module Test

struct Handle: not Copyable {
    var fd: lang.i64
}

struct Wrapper {
    var handle: Handle

    deinit {}
}
