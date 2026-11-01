// test: execution
// stdlib: true
// expect-exit: 32
//
// Same-label methods are legal when each witnesses a different instantiation
// of one parameterized protocol. Witness binding must match each requirement
// by its SUBSTITUTED signature (`Pick[V]` wants `pick(value: V)`), never by
// declaration order: the `Pick[Float64]` witness is declared first. Generic
// calls through each instantiation reach their own witness (3 and 2 → 32).

module Test

protocol Pick[T] {
    func pick(value: T) -> Int64
}

struct V: Pick[Float64], Pick[V] {
    var n: Int64
    func pick(value: Float64) -> Int64 { 2 }
    func pick(value: V) -> Int64 { 3 }
}

func viaSelf[X](x: X, y: X) -> Int64 where X: Pick[X] { x.pick(y) }
func viaFloat[X](x: X, f: Float64) -> Int64 where X: Pick[Float64] { x.pick(f) }

@main
func main() -> Int64 {
    viaSelf(V(n: 0), V(n: 1)) * 10 + viaFloat(V(n: 0), 1.5)
}
