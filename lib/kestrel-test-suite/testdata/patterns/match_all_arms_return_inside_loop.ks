// test: execution
// backends: cranelift,llvm
// expect-stdout: drop 3\ndrop 5\n

// When every arm of a `match` returns, nothing reaches the code after it.
// Inside a loop (and under an `if`), the dead merge used to be treated as a
// live exit: the loop's back edge got the wrong arguments (a backend verifier
// error) or the consumed scrutinee was dropped again (an OSSA error).

module Test

struct R: not Copyable {
    var v: Int64;
    deinit { println("drop \(self.v)"); }
}

func first(consuming o: Optional[R]) -> Int64 {
    for i in 0..<3 {
        match o {
            .Some(r) => { return r.v },
            .None => { return 0 }
        }
    }
    return -1
}

func second(consuming o: Optional[R]) -> Int64 {
    for i in 0..<3 {
        if i == 1 {
            match o {
                .Some(r) => { return r.v },
                .None => { return 0 }
            }
        }
    }
    return -1
}

@main
func main() -> lang.i64 {
    if first(.Some(R(v: 3))) != 3 { return 1 }
    if second(.Some(R(v: 5))) != 5 { return 2 }
    0
}
