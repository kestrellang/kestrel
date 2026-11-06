// test: execution
// stdlib: true
// backends: cranelift,llvm
// expect-stdout: 14 4 | 11 11 | 3 3 7 99 | 2 7\n

// A protocol can require a place: `var item: Item { ref mutating ref }`.
// Generic code reads, writes, and calls mutating methods through it, and
// the conformer's own `ref` / `mutating ref` accessors (or a stored field,
// lent in place) witness it.
module Test

import std.collections.(Array)

protocol Holder {
    type Item
    var item: Item { ref mutating ref }
}

protocol Reader {
    type Item
    var item: Item { ref }
}

// Witnessed by accessors.
struct Cell: Holder {
    type Item = Int64
    var v: Int64
    var item: Int64 {
        ref { self.v }
        mutating ref { self.v }
    }
}

// Witnessed by stored fields, plain and generic.
struct Plain: Holder {
    type Item = Int64
    var item: Int64
}

extend Plain: Reader {}

struct Gen[T]: Holder {
    type Item = T
    var item: T
}

struct Bag: Holder {
    type Item = Array[Int64]
    var xs: Array[Int64]
    var item: Array[Int64] {
        ref { self.xs }
        mutating ref { self.xs }
    }
}

func bump[H](h: &mutating H) where H: Holder, H.Item = Int64 { h.item += 10 }

func read[H](h: H) -> H.Item where H: Holder, H.Item: Copyable { h.item }

func fetch[H](h: H) -> Int64 where H: Reader, H.Item = Int64 { h.item }

func push[H](h: &mutating H) where H: Holder, H.Item = Array[Int64] { h.item.append(7) }

func count[H](h: &H) -> Int64 where H: Holder, H.Item = Array[Int64] { h.item.count }

func last[H](h: &H) -> Int64 where H: Holder, H.Item = Array[Int64] {
    let xs = h.item;
    xs(xs.count - 1)
}

@main
func main() -> lang.i64 {
    var c = Cell(v: 4);
    bump(c);
    var p = Plain(item: 1);
    bump(p);
    var b = Bag(xs: [1, 2]);
    push(b);
    var g = Gen(item: [1]);
    push(g);
    // A concrete subscript write through a place property.
    b.item(0) = 99;
    print("\(c.v) \(read(Cell(v: 4))) | \(p.item) \(fetch(p)) | \(count(b)) \(b.xs.count) \(last(b)) \(b.xs(0)) | \(count(g)) \(last(g))\n");
    0
}
