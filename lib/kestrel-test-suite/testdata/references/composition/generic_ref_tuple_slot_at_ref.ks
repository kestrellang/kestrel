// test: execution
// stdlib: true
// expect-exit: 7

// A generic tuple with a `&T` slot, filled from a borrowed payload at
// `T = &Point`: `&&Point` is `&Point`, so the slot must hold the payload's
// pointer, not the address of the payload slot.

module Test

struct Point {
    var x: Int64
}

func tagged[T](o: Optional[T]) -> Optional[(&T, Int64)] where T: not Static {
    match o {
        .Some(v) => {
            let r = &v;
            .Some((r, 1))
        },
        .None => .None
    }
}

@main
func main() -> Int64 {
    let point: Point? = .Some(Point(x: 6));
    let borrowed = point.asRef();
    match tagged(borrowed) {
        .Some((r, n)) => r.x + n,
        .None => 100
    }
}
