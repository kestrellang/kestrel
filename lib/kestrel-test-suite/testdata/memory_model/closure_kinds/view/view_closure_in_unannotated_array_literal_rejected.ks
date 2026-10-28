// test: diagnostics

// An un-annotated array literal defaults to `Array[E]`, which requires
// `E: Static` exactly like the annotation `Array[() -> Int64]` does. A view
// closure is not Static, so `[{ k }]` is rejected. It used to slip through,
// escape the frame, and be called after `k` was gone.

module Test

func mk(n: Int64) -> () -> Int64 {
    let k: Int64 = n * 3;
    let arr = [{ k }]; // ERROR: does not satisfy constraint
    arr(0)
}
