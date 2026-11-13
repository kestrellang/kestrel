// test: execution
// stdlib: true
// backends: cranelift,llvm
// expect-stdout: cow c 10 3 d 1 2\nrc s 5 t 5\nowned 34 3\n

// Every box exposes its payload as `.value`. On the value-semantic boxes
// (`CowBox`, `OwnedBox`) it is an in-place place: a write forks a shared
// `CowBox` first. On the shared `RcBox` `.value` is read-only and writes go
// through `mutatingValue`, which every handle sees.
module Test

struct P: Cloneable {
    var x: Int64
    var y: Int64
    func clone() -> P { P(x: self.x, y: self.y) }
}

func bump(mutating n: Int64) { n += 1 }

@main
func main() -> lang.i64 {
    var c: CowBox[P] = CowBox(P(x: 1, y: 2));
    let d = c;
    c.value.x = 10;
    bump(c.value.y);
    let r = &c.value;
    println("cow c \(r.x) \(c.value.y) d \(d.value.x) \(d.value.y)");

    let s: RcBox[P] = RcBox(P(x: 1, y: 2));
    let t = s;
    s.mutatingValue.x = 5;
    println("rc s \(s.value.x) t \(t.value.x)");

    var o: OwnedBox[P] = OwnedBox(P(x: 3, y: 4));
    o.value.y += 30;
    let copied = o.value;
    println("owned \(o.value.y) \(copied.x)");
    0
}
