// test: execution
// stdlib: true
// expect-stdout: drop 1\nread 1\nwork\nend 1\ndrop 1\n

// Without `@observabledrop`, an implicit copy is dropped right after its last
// read (copy lifetime passes). The original keeps its scope-end drop. `b`'s
// last read is `b.id` — the Int64 is copied out before `println` runs — so
// "drop 1" prints before "read 1".
module Test

struct Noisy: Cloneable {
    var id: Int64
    func clone() -> Noisy { Noisy(id: self.id) }
    deinit { println("drop \(self.id)"); }
}

@main
func main() -> lang.i64 {
    let a = Noisy(id: 1);
    let b = a;                    // implicit copy (clone)
    println("read \(b.id)");      // last read of b is `b.id`
    println("work");
    println("end \(a.id)");       // keeps the original alive and copied
    0
}
