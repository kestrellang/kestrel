// test: execution
// stdlib: true
// expect-exit: 0
//
// Pattern parameters through every use of a pattern extension: a
// conformance used generically (witness dispatch), a static requirement
// called through a type param, a static method on a concrete instantiation,
// a computed property, an associated type, and specificity against a
// generic `extend Box[T]` of the same protocol.

module Test

protocol P { func n() -> Int64 }
extend Int64: P { func n() -> Int64 { self } }
extend Bool: P { func n() -> Int64 { if self { 1 } else { 0 } } }

protocol Make { static func make(n: Int64) -> Self }
extend Int64: Make { static func make(n: Int64) -> Int64 { n } }

protocol Container {
    type Element
    func first() -> Element
}

protocol Tag { func tag() -> Int64 }

struct Box[T] { var v: T }

extend Box[(A, B)]: P where A: P, B: P {
    func n() -> Int64 { self.v.0.n() * 10 + self.v.1.n() }
}

extend Box[(A, B)]: Make where A: Make, B: Make {
    static func make(n: Int64) -> Box[(A, B)] { Box(v: (A.make(n), B.make(n + 1))) }
}

extend Box[(A, B)] where A: P {
    static func weigh(a: A) -> Int64 { a.n() }
    var second: B { self.v.1 }
}

extend Box[(A, B)]: Container {
    type Element = A
    func first() -> A { self.v.0 }
}

extend Box[T]: Tag { func tag() -> Int64 { 1 } }
extend Box[(A, B)]: Tag { func tag() -> Int64 { 2 } }

func total[T](x: T) -> Int64 where T: P { x.n() }
func build[T](n: Int64) -> T where T: Make { T.make(n) }
func head[C](c: C) -> C.Element where C: Container { c.first() }
func tagOf[X](x: X) -> Int64 where X: Tag { x.tag() }

@main
func main() -> Int64 {
    if total(Box(v: (3, true))) != 31 { return 1 }
    let b: Box[(Int64, Int64)] = build(4);
    if b.v.0 != 4 or b.v.1 != 5 { return 2 }
    if Box[(Int64, Bool)].weigh(5) != 5 { return 3 }
    if Box(v: (true, 9)).second != 9 { return 4 }
    let x: Int64 = head(Box(v: (6, true)));
    if x != 6 { return 5 }
    if tagOf(Box(v: 5)) != 1 { return 6 }
    if tagOf(Box(v: (1, 2))) != 2 { return 7 }
    0
}
