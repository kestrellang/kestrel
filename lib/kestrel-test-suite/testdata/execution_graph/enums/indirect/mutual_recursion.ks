// test: execution
// stdlib: true
// expect-exit: 0
//
// Recursion through a struct (`Tree` ↔ `Forest`), through `Optional`, and
// between two indirect enums (`Expr` ↔ `Stmt`) all box the recursive cases.

module Test

import std.numeric.(Int64)

indirect enum Tree {
    case Leaf(Int64)
    case Branch(Forest)

    func sum() -> Int64 {
        match self {
            .Leaf(v) => v,
            .Branch(forest) => forest.left.sum() + forest.right.sum()
        }
    }
}

struct Forest {
    var left: Tree
    var right: Tree
}

indirect enum Chain {
    case Link(Int64, Optional[Chain])

    func length() -> Int64 {
        match self {
            .Link(_, .None) => 1,
            .Link(_, .Some(next)) => 1 + next.length()
        }
    }
}

indirect enum Expr {
    case Num(Int64)
    case Block(Stmt)

    func eval() -> Int64 {
        match self {
            .Num(n) => n,
            .Block(stmt) => stmt.eval()
        }
    }
}

indirect enum Stmt {
    case Done
    case Add(Expr, Stmt)

    func eval() -> Int64 {
        match self {
            .Done => 0,
            .Add(expr, rest) => expr.eval() + rest.eval()
        }
    }
}

@main
func main() -> Int64 {
    let tree = Tree.Branch(Forest(left: .Leaf(1), right: .Branch(Forest(left: .Leaf(2), right: .Leaf(3)))));
    if tree.sum() != 6 { return 1 }
    let chain = Chain.Link(1, .Some(.Link(2, .Some(.Link(3, .None)))));
    if chain.length() != 3 { return 2 }
    let program = Expr.Block(.Add(.Num(1), .Add(.Block(.Add(.Num(2), .Done)), .Done)));
    if program.eval() != 3 { return 3 }
    0
}
