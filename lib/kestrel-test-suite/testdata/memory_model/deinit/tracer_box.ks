// test: mir
// stdlib: false
// mir-filter: Test.use_handle

// Tracer test for the greenfield memory-model rewrite (Stage 1).
//
// Verifies the new `kestrel-ownership` crate is wired into the compiler
// pipeline and the legacy `Deinit`/`DeinitIf` statements have been
// rewritten to `Drop`/`DropIf` by `drop_elab`. The snapshot must contain
// `drop` (not `deinit`) before the return.

module Test

@builtin(.Copyable)
protocol Copyable {}

struct Handle: not Copyable {
    var fd: lang.i64
    deinit {}
}

func use_handle() {
    let h = Handle(fd: 7);
}
