// test: execution
// stdlib: true
// backends: cranelift,llvm
// expect-stdout: drop 1\n

// Printing reaches `write` only through generic and protocol calls
// (println -> writeLine[W] -> W.write with W = Stdout). The post-mono walk sees
// the resolved calls, so a deinit that only prints is caught too.
module Test

struct Noisy: Cloneable { // WARN: `Cloneable` type `Noisy` has a `deinit` that calls `println`
    var id: Int64
    func clone() -> Noisy { Noisy(id: self.id) }
    deinit { println("drop \(self.id)"); }
}

@main
func main() -> lang.i64 {
    let n = Noisy(id: 1);
    0
}
