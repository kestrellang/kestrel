// test: diagnostics
// stdlib: false

module Test

struct S: not Copyable {
    @dummy
    deinit {}
}
