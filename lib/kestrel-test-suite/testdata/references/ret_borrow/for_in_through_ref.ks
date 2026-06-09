// test: execution
// stdlib: true
// backends: cranelift,llvm
// skip: stage1 — needs ref returns end-to-end (S1+M6)

// for-in is receiver see-through: iterating a `&Array[T]` borrows the
// array in place (iter() is borrowed-self).
module Test

struct Holder {
    var items: Array[Int64]
    func view() -> &Array[Int64] { self.items }
}

@main
func main() -> lang.i64 {
    let h = Holder(items: [1, 2, 3]);
    var sum = 0;
    for x in h.view() {
        sum = sum + x;
    }
    if sum != 6 { return 1; }
    0
}
