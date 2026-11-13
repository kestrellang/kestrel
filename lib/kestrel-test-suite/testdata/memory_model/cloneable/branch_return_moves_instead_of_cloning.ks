// test: execution
// stdlib: true
// expect-stdout: 1 2 3 4\nclones: 0\n

module Test

// Returning a local from an `if`/`match` branch moves it: before, each of
// these cloned the value and then dropped the original.
var clones: Int64 = 0;

struct Counted: Cloneable {
    var n: Int64
    func clone() -> Counted { clones += 1; Counted(n: self.n) }
}

func branchLocal(c: Bool) -> Counted {
    if c { var x = Counted(n: 1); x.n = 1; x } else { Counted(n: 0) }
}

func branchLet(c: Bool) -> Counted {
    if c { let x = Counted(n: 2); x } else { Counted(n: 0) }
}

func matchArm(n: Int64) -> Counted {
    match n { 0 => Counted(n: 0), _ => { var x = Counted(n: 0); x.n = n; x } }
}

func outerVar(c: Bool) -> Counted {
    var x = Counted(n: 4);
    if c { x } else { Counted(n: 0) }
}

@main
func main() {
    let a = branchLocal(true);
    let b = branchLet(true);
    let c = matchArm(3);
    let d = outerVar(true);
    print("\(a.n) \(b.n) \(c.n) \(d.n)\nclones: \(clones)\n");
}
