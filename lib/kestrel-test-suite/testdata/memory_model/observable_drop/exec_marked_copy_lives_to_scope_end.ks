// test: execution
// stdlib: true
// expect-stdout: read 1\nwork\nend 1\ndrop 1\ndrop 1\n

// A copy of an `@observabledrop` type keeps its scope-end drop: it is dropped
// with the original at the end of the scope, not right after its last read.
module Test

@observabledrop
struct Noisy: Cloneable {
    var id: Int64
    func clone() -> Noisy { Noisy(id: self.id) }
    deinit { println("drop \(self.id)"); }
}

@main
func main() -> lang.i64 {
    let a = Noisy(id: 1);
    let b = a;                    // implicit copy (clone)
    println("read \(b.id)");      // last read of b: still dropped at scope end
    println("work");
    println("end \(a.id)");       // keeps the original alive and copied
    0
}
