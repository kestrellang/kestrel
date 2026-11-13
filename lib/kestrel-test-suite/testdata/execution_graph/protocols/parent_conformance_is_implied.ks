// test: execution
// stdlib: true
// expect-stdout: hi Bob Bob Bob\nhello Bob\nfalse true\n

// Conforming to a protocol conforms to its parents too: `Bob: Loud` is also
// `Greeter` and `Named` without listing them. Parent requirements still
// have to be implemented (see parent_requirement_still_required.ks), and
// generic bounds, existentials and parent extensions all see the parents.
module Test

protocol Named { func name() -> String }
extend Named { func hello() -> String { "hello \(self.name())" } }
protocol Greeter: Named { func greet() -> String }
protocol Loud: Greeter { func shout() -> String }

struct Bob: Loud {
    func name() -> String { "Bob" }
    func greet() -> String { "hi" }
    func shout() -> String { "HEY" }
}

// `Comparable: Equatable`, so `==` comes with `Comparable` alone.
struct V: Comparable {
    var x: Int64
    func isEqual(to other: V) -> Bool { self.x == other.x }
    func compare(other: V) -> Ordering { self.x.compare(other.x) }
}

func show[T](x: T) -> String where T: Named { x.name() }
func viaAny(x: any Named) -> String { x.name() }

@main
func main() -> Int64 {
    let b = Bob();
    println("\(b.greet()) \(show(b)) \(viaAny(b)) \(b.name())");
    println(b.hello());
    let p = V(x: 1);
    let q = V(x: 2);
    println("\(p == q) \(p < q)");
    0
}
