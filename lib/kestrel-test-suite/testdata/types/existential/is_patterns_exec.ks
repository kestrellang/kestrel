// test: execution
// stdlib: true
// expect-exit: 0
//
// `is` patterns: `is T`, `c is T`, `is T { .. }` and `r is T { .. }` in
// `match`, `if let`, `guard let` and `while let`; `x @ is T` binds the whole
// existential (design §8b).

module Test

import std.numeric.(Int64)

protocol Shape {
    func area() -> Int64
}

struct Circle { var r: Int64; }
extend Circle: Shape {
    public func area() -> Int64 { 3 * self.r * self.r }
}

struct Square { var side: Int64; }
extend Square: Shape {
    public func area() -> Int64 { self.side * self.side }
}

struct Rect { var w: Int64; var h: Int64; }
extend Rect: Shape {
    public func area() -> Int64 { self.w * self.h }
}

func classify(s: any Shape) -> Int64 {
    match s {
        is Circle => 1,
        q is Square => q.side * 10,
        is Rect { w: 0, .. } => 2,
        r is Rect { w, .. } => r.h * 100 + w,
        _ => 0,
    }
}

func sideOf(s: any Shape) -> Int64 {
    guard let q is Square = s else { return -1 }
    q.side
}

@main
func main() -> Int64 {
    if classify(Circle(r: 5)) != 1 { return 1 }
    if classify(Square(side: 3)) != 30 { return 2 }
    if classify(Rect(w: 0, h: 9)) != 2 { return 3 }
    if classify(Rect(w: 2, h: 3)) != 302 { return 4 }
    let s: any Shape = Circle(r: 7);
    if let c is Circle = s { if c.r != 7 { return 5 } } else { return 6 }
    if sideOf(Square(side: 4)) != 4 { return 7 }
    if sideOf(s) != -1 { return 8 }
    match s {
        whole @ is Circle => { if whole.area() != 147 { return 9 } },
        _ => { return 10 },
    }
    var xs: Array[any Shape] = [Square(side: 1), Square(side: 2), Circle(r: 1)];
    var i: Int64 = 0;
    var total: Int64 = 0;
    while let q is Square = xs(i) { total = total + q.side; i = i + 1; }
    if total != 3 { return 11 }
    0
}
