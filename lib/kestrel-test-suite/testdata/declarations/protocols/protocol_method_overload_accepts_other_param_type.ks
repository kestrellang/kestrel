// test: execution
// stdlib: true
// expect-exit: 22
//
// With two same-label witnesses (`V: Pick[Float64], Pick[V]`), a DIRECT call
// picks the overload by argument type: `pick(1.5)` is the Float64 witness and
// `pick(V)` the `Pick[V]` one, whatever the declaration order (1+1, 2 → 22).

module Test

protocol Pick[T] {
    func pick(value: T) -> Int64
}

struct V: Pick[Float64], Pick[V] {
    var n: Int64
    func pick(value: Float64) -> Int64 { 1 }
    func pick(value: V) -> Int64 { 2 }
}

@main
func main() -> Int64 {
    let f: Float64 = 1.5;
    let byFloat = V(n: 0).pick(f) + 1;
    let bySelf = V(n: 0).pick(V(n: 1));
    byFloat * 10 + bySelf
}
