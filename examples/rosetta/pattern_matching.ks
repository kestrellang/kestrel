// Rosetta Code: Pattern matching (red-black tree insertion)
// https://rosettacode.org/wiki/Pattern_matching
module Main

enum Color {
    case Red
    case Black
}

indirect enum Tree {
    case Empty
    case Node(color: Color, left: Tree, value: Int64, right: Tree)
}

func node(c: Color, l: Tree, v: Int64, r: Tree) -> Tree { .Node(color: c, left: l, value: v, right: r) }

// Okasaki's balance: the four red-red shapes all rewrite to the same tree.
func balance(t: Tree) -> Tree {
    match t {
        .Node(.Black, .Node(.Red, .Node(.Red, a, x, b), y, c), z, d) =>
            node(.Red, node(.Black, a, x, b), y, node(.Black, c, z, d)),
        .Node(.Black, .Node(.Red, a, x, .Node(.Red, b, y, c)), z, d) =>
            node(.Red, node(.Black, a, x, b), y, node(.Black, c, z, d)),
        .Node(.Black, a, x, .Node(.Red, .Node(.Red, b, y, c), z, d)) =>
            node(.Red, node(.Black, a, x, b), y, node(.Black, c, z, d)),
        .Node(.Black, a, x, .Node(.Red, b, y, .Node(.Red, c, z, d))) =>
            node(.Red, node(.Black, a, x, b), y, node(.Black, c, z, d)),
        _ => t,
    }
}

func ins(t: Tree, x: Int64) -> Tree {
    match t {
        .Empty => node(.Red, .Empty, x, .Empty),
        .Node(c, l, v, r) => {
            if x < v { balance(node(c, ins(l, x), v, r)) }
            else if x > v { balance(node(c, l, v, ins(r, x))) }
            else { t }
        },
    }
}

func insert(t: Tree, x: Int64) -> Tree {
    match ins(t, x) {
        .Node(_, l, v, r) => node(.Black, l, v, r),
        .Empty => .Empty,
    }
}

func show(t: Tree) -> String {
    match t {
        .Empty => "E",
        .Node(c, l, v, r) => {
            let tag = match c { .Red => "R", .Black => "B" };
            "(\(tag) \(show(l)) \(v) \(show(r)))"
        },
    }
}

func depth(t: Tree) -> Int64 {
    match t {
        .Empty => 0,
        .Node(_, l, _, r) => {
            let a = depth(l);
            let b = depth(r);
            1 + (if a > b { a } else { b })
        },
    }
}

@main
func main() {
    var t = Tree.Empty;
    for x in 1..=10 { t = insert(t, x); }
    println(show(t));
    var big = Tree.Empty;
    for x in 1..=1000 { big = insert(big, x); }
    println("depth after inserting 1..1000 in order: \(depth(big))");
}
