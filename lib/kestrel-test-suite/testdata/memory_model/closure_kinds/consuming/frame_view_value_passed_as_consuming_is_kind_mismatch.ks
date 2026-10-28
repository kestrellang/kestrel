// test: diagnostics
// stdlib: true

// docs/design/closures.md, "Passing: What Fits Where": normal -> `consuming`
// is "✗ frame view is not owned". An already-built normal closure value holds
// views of the frame and cannot acquire an owned, returnable environment by
// coercion, so the argument is a kind mismatch (E624).
module Test

import std.numeric.Int64

func onDone(consuming f: consuming () -> ()) { f(); }

func main() -> lang.i64 {
    var x: Int64 = 1;
    // Inferred normal kind: read-only body, frame views.
    let view = { () in let _ = x; };
    onDone(view); // ERROR(E624)
    x = 2;
    let _ = x;
    0
}
