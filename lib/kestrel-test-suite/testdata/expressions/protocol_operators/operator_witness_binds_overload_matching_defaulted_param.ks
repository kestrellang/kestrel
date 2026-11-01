// test: execution
// stdlib: true
// expect-exit: 221
//
// `V: Addable` leaves `Other` to its default (`Other = Self`), beside an
// explicit `Addable[Float64]`. Each `add` witnesses one instantiation: `V + V`
// (direct and generic) must call `add(other: V)` (2), and `V + 1.5` the
// Float64 one (1), though that is declared first: 2*100 + 2*10 + 1.

module Test

struct V: Addable, Addable[Float64] {
    var x: Int64
    type Addable[V].Output = Int64
    type Addable[Float64].Output = Int64
    static var zero: V { V(x: 0) }
    consuming func add(consuming other: Float64) -> Int64 { 1 }
    consuming func add(consuming other: V) -> Int64 { 2 }
}

func viaProtocol[T](a: T, b: T) -> T.Output where T: Addable { a + b }

@main
func main() -> Int64 {
    let direct = V(x: 1) + V(x: 2);
    let generic = viaProtocol(V(x: 1), V(x: 2));
    let float = V(x: 1) + 1.5;
    direct * 100 + generic * 10 + float
}
