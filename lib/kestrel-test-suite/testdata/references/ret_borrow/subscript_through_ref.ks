// test: execution
// stdlib: true
// backends: cranelift,llvm

// Paren-subscript on a ref-typed receiver: `h.view()(1)` peels the ref
// returned by the accessor and subscripts the borrowed Array in place.
module Test

struct Holder {
    var items: Array[Int64]
    func view() -> &Array[Int64] { self.items }
}

@main
func main() -> lang.i64 {
    let h = Holder(items: [10, 20, 30]);
    if h.view()(1) != 20 { return 1; }
    0
}
