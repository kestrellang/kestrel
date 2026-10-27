// test: execution
// backends: cranelift,llvm
// expect-stdout: take 1\ndrop 1\ntake 2\ndrop 2\nend\ntake 7\ndrop 7\nend 2\ndrop 9\n

// `while let .Some(r) = o` moves `o`'s payload out each iteration. Whether the
// body refills `o` or breaks, the loop exit must know `o`'s real state (it
// used to assume `o` was still initialized and destroy it a second time).

module Test

struct R: not Copyable {
    var id: Int64;
    deinit { println("drop \(self.id)"); }
}

func take(consuming r: R) {
    println("take \(r.id)");
    let gone = r.id;
    deinit r;
}

@main
func main() -> lang.i64 {
    var o: Optional[R] = .Some(R(id: 1));
    var n = 1;
    while let .Some(r) = o {
        take(r);
        n = n + 1;
        o = if n <= 2 { .Some(R(id: n)) } else { .None };
    }
    println("end");

    var p: Optional[R] = .Some(R(id: 7));
    while let .Some(r) = p {
        take(r);
        break;
    }
    println("end 2");
    let keep = R(id: 9);
    0
}
