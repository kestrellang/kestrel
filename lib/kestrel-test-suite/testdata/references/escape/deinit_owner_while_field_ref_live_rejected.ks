// test: diagnostics
// stdlib: true

// `deinit c;` while a ref binding into `c` is still live. Consuming the owner
// in the same position is E498 (intra_block_consume_while_borrowed.ks), but
// `DestroyAddr` does not run the borrow check that `Take` does (mir verify.rs),
// and the analyzer's `deinit` freeze check covers closures only. `rc` then
// reads the freed String. Found in the 2026-10 architecture review at
// 9767d2dc: valgrind invalid read/write.
// EXPECTED TO FAIL until destroying a place checks for live refs into it.
module Test

struct Res: not Copyable {
    var name: String
}

func test() -> Int64 {
    var c = Res(name: "resource name, long enough to live on the heap");
    let rc = &c.name;
    deinit c; // ERROR(E498)
    rc.bytes.count
}
