// test: execution
// stdlib: true
// expect-stdout: if 7\nif 0\ndrop 100\nmatch 1\nmatch 0\ndrop 200\nend\ndrop 0\ndrop 200\ndrop 0\ndrop 100\n

module Test

// A `var` returned from one branch of a return-position `if`/`match` is
// moved out on that path and must still be dropped on the others. It gets a
// drop flag for that, even when the type is Copyable.
struct Owner {
    var n: Int64
    deinit { println("drop \(self.n)") }
}

func viaIf(n: Int64) -> Owner {
    var o = Owner(n: 0);
    o.n = 100;
    if n > 5 { o } else { Owner(n: 0) }
}

func viaMatch(n: Int64) -> Owner {
    var o = Owner(n: 0);
    o.n = 200;
    match n { 0 => Owner(n: 0), _ => o }
}

@main
func main() {
    println("if 7"); let a = viaIf(7);
    println("if 0"); let b = viaIf(0);
    println("match 1"); let c = viaMatch(1);
    println("match 0"); let d = viaMatch(0);
    println("end");
}
