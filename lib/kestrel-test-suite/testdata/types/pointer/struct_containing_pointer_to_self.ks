// test: diagnostics
// stdlib: false

module Test

struct Node {
    let value: lang.i64
    let next: lang.ptr[Node]
}
