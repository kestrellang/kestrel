// test: diagnostics
// stdlib: false

// docs/design/closures.md — the passing table: normal (frame view) -> escaping
// is "✗ frame-bound". An existing view-kind closure VALUE can never be laundered
// into an owning position, by annotation or by argument; only a literal built
// directly against an `escaping` expected type gets an owned environment.
module Test

func store(f: escaping () -> lang.i64) {}

func test() {
    var x: lang.i64 = 1;
    let view = { x };                                   // inferred normal: holds a frame view
    let owned: escaping () -> lang.i64 = view; // ERROR(E624)
    store(view); // ERROR(E624)
}
