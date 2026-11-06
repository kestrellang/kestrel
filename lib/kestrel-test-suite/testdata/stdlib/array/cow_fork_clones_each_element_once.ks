// test: execution
// stdlib: true
// backends: cranelift,llvm
// expect-stdout: append on shared array:\n  clone 1\n  clone 2\nsnap 2 a 3\n

// Mutating a shared Array forks its storage: each element must be cloned
// exactly once. The CowBox barrier used to materialize the storage by value
// several times (getValue() copies + write()/setValue() round trip), cloning
// every element 4x per fork.
module Test

struct Tracked: Cloneable {
    var id: Int64;
    func clone() -> Tracked {
        println("  clone \(self.id)");
        Tracked(id: self.id)
    }
}

@main
func main() -> lang.i64 {
    var a: Array[Tracked] = Array();
    a.append(Tracked(id: 1));
    a.append(Tracked(id: 2));
    let snap = a;
    println("append on shared array:");
    a.append(Tracked(id: 3));     // must fork: snap is used below
    println("snap \(snap.count) a \(a.count)");
    0
}
