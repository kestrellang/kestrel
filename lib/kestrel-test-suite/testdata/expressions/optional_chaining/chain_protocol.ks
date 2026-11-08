// test: execution
// stdlib: true
// expect-exit: 0

// `?.` works on any `OptionalChain` type: a user type, and a generic base.

module Test

import std.core.(OptionalChain)

protocol HasX {
    var x: Int64 { get }
}

struct Point: HasX {
    var x: Int64
}

struct Maybe: OptionalChain {
    type Wrapped = Point
    var value: Point
    var present: Bool

    func chainBorrow() -> Optional[&Point] {
        if self.present == false { return .None }
        let r = &self.value;
        let o: Optional[&Point] = .Some(r);
        o
    }
    mutating func chainMutate() -> Optional[&mutating Point] {
        if self.present == false { return .None }
        let r = &mutating self.value;
        let o: Optional[&mutating Point] = .Some(r);
        o
    }
    consuming func chainTake() -> Optional[Point] {
        if self.present == false { return .None }
        .Some(self.value)
    }
}

func firstX[O](o: O) -> Int64? where O: OptionalChain, O.Wrapped: HasX {
    o?.x
}

@main
func main() -> lang.i64 {
    var m = Maybe(value: Point(x: 3), present: true);
    if m?.x != .Some(3) { return 1 }
    m?.x = 8;
    if m.value.x != 8 { return 2 }
    let absent = Maybe(value: Point(x: 3), present: false);
    if absent?.x != .None { return 3 }
    if firstX(m) != .Some(8) { return 4 }
    let o: Point? = .Some(Point(x: 5));
    if firstX(o) != .Some(5) { return 5 }
    0
}
