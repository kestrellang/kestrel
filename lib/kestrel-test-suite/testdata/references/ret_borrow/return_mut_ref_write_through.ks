// test: execution
// stdlib: true
// backends: cranelift,llvm

// Write-through: a mutating method called THROUGH `&mutating` mutates the
// element in place — no copy, no write-back.
module Test

struct Counter {
    var n: Int64
    mutating func bump() { self.n = self.n + 1; }
}

@main
func main() -> lang.i64 {
    var arr = [Counter(n: 0), Counter(n: 5)];
    arr.mutableAt(index: 0).bump();
    if arr(0).n != 1 { return 1; }
    if arr(1).n != 5 { return 2; }
    0
}
