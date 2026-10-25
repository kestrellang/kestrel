// test: diagnostics
// stdlib: false

module Test

@inline
func free() {}

struct S: not Copyable {
    var stored: lang.i64

    @inline(.Always)
    init() { self.stored = 0 }

    @inline(.Never)
    func method() {}

    @inline
    var computed: lang.i64 { self.stored }

    @inline
    subscript(i: lang.i64) -> lang.i64 { get { i } }

    @inline
    deinit {}
}

protocol P {
    func required()
}

extend P {
    @inline
    func provided() {}
}
