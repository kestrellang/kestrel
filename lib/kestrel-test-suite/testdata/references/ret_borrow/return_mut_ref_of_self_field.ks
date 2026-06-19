// test: execution
// stdlib: true
// backends: cranelift,llvm

// A `&mutating` return of a direct FIELD projection on a `mutating`
// receiver roots at Param(self) — the field address inherits the
// receiver's provenance (it used to self-root `Local` and fail E494).
// Writes through the returned ref must land in the original storage.
module Test

struct Box {
    var v: Int64

    mutating func view() -> &mutating Int64 {
        self.v
    }
}

@main
func main() -> lang.i64 {
    var b = Box(v: 1);
    b.view() = 5;
    if b.v != 5 { return 1; }
    b.view() = b.view() + 2;
    if b.v != 7 { return 2; }
    0
}
