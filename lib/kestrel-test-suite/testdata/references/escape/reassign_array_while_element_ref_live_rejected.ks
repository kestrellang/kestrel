// test: diagnostics
// stdlib: true

// A ref binding into an array element, then the array is reassigned while the
// ref is live. `at:` returns a ref into the HEAP buffer, so `xs = []` frees the
// storage `e` points into; the ref is rooted at the still-live local `xs`, so
// the frame-granular escape check is satisfied, and E498 only guards consuming
// the root. Found in the 2026-10 architecture review at 9767d2dc: segfault.
//
// E498 ("cannot consume ... while a reference into it is live") is the closest
// existing rule — reassignment destroys the old value. If the fix allocates a
// dedicated code, update the annotation.
// EXPECTED TO FAIL until destroying or replacing a ref's root is checked.
module Test

func test() -> Int64 {
    var xs = ["element zero, long enough to live on the heap"];
    let e = &xs(at: 0);
    xs = []; // ERROR(E498)
    e.bytes.count
}
