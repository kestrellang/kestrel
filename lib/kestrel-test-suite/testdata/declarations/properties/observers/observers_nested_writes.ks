// test: execution
// stdlib: true
// backends: cranelift,llvm

// Every write form that lands INSIDE an observed property (a stored field
// of it, an element of one of its arrays, a mutating call on one of its
// fields) runs the property's observers once. One line per write form.
module Test

import std.numeric.(Int64)
import std.collections.(Array)

struct Counter {
    var n: Int64
    mutating func bump() { self.n += 1; }
    subscript(i: Int64) -> Int64 {
        modifying get { self.n += i; self.n }
    }
}

struct Pos {
    var x: Int64
    var items: Array[Int64]
    var c: Counter
}

struct O {
    var hits: Int64
    var pos: Pos {
        didSet { self.hits += 1; }
    }
}

func bumpIt(mutating c: Counter) { c.bump(); }

@main
func main() -> lang.i64 {
    var o = O(hits: 0, pos: Pos(x: 0, items: [1, 2], c: Counter(n: 0)));
    o.pos.x = 3;                       // stored field assignment
    if o.hits != 1 { return 1; }
    o.pos.x += 1;                      // compound assignment
    if o.hits != 2 { return 2; }
    o.pos.items(at: 0) = 9;            // subscript store on a field
    if o.pos.items(at: 0) != 9 { return 3; }
    if o.hits != 3 { return 4; }
    o.pos.items(at: 1) += 1;           // subscript compound assignment
    if o.pos.items(at: 1) != 3 { return 5; }
    if o.hits != 4 { return 6; }
    o.pos.c.bump();                    // mutating method on a field
    if o.pos.c.n != 1 { return 7; }
    if o.hits != 5 { return 8; }
    bumpIt(o.pos.c);                   // `mutating` argument
    if o.pos.c.n != 2 { return 9; }
    if o.hits != 6 { return 10; }
    let y = o.pos.c(5);                // `modifying get` subscript read
    if y != 7 { return 11; }
    if o.hits != 7 { return 12; }
    o.pos.items.append(4);             // mutating method on a field's array
    if o.pos.items.count != 3 { return 13; }
    if o.hits != 8 { return 14; }
    0
}
