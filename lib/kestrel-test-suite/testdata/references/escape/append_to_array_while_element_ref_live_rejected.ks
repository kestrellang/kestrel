// test: diagnostics
// stdlib: true

// A ref binding into an array element, then the array grows while the ref is
// live. `at:` returns a ref into the HEAP buffer; `append` reallocates it, so
// `e` dangles. Stage-1 refs were expression-scoped, so no mutation could
// intervene (references-gaps.md); named ref bindings removed that guarantee
// and nothing replaced it. Found in the 2026-10 architecture review at
// 9767d2dc: reading `e` after 1000 appends printed a garbage value.
//
// E498 is the closest existing rule; a mutating call on the root while a
// heap-interior ref is live may well get its own code. If so, update the
// annotation. The alternative fix — narrowing named ref bindings to stack
// places — would turn this into a rejection at the `let` instead.
// EXPECTED TO FAIL until mutating a ref's root is checked.
module Test

func test() -> Int64 {
    var xs: [Int64] = [1, 2, 3];
    let e = &xs(at: 0);
    var i: Int64 = 0;
    while i < 1000 {
        xs.append(i); // ERROR(E498)
        i = i + 1;
    }
    e
}
