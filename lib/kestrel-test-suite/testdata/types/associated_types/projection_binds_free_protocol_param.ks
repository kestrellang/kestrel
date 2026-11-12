// test: execution
// stdlib: true
// expect-exit: 0
// expect-stdout: 5\n5\n
//
// `extend Range[Int64]: VIdx[W]` leaves `W` free: only the protocol arg
// names it. Projecting `I.Output where I: VIdx[Int64]` (a concrete
// protocol arg) must bind `W` to `Int64`, giving `Box[Int64]`.

module Test

struct Box[W] { var w: W }

protocol VIdx[W] {
    type Output
    @sugarOnly
    static func read(consuming index: Self, view: W) -> Output
}

extend Range[Int64]: VIdx[W] {
    type Output = Box[W]
    static func read(consuming index: Range[Int64], view: W) -> Box[W] { Box(w: view) }
}

func fetch[I](index: I, v: Int64) -> I.Output where I: VIdx[Int64] { I.read(index, v) }

func fetchAny[I, X](index: I, v: X) -> I.Output where I: VIdx[X] { I.read(index, v) }

@main
func main() {
    let b = fetch(2..<5, 5);
    print("\(b.w)\n");
    let c = fetchAny(4..<5, 5);
    print("\(c.w)\n");
}
