// test: diagnostics
// stdlib: true

// `deinit c;` while a ref binding into `c` is still live is E498, the same
// rule as consuming `c`. The ref is taken through a field projection, so the
// verifier traces it to `c`'s storage. Only a source-level `deinit` is
// checked; compiler cleanup at scope exit under a live ref is the escape
// E494 already reports.

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
