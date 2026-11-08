// test: execution
// stdlib: true
// expect-exit: 0
//
// Consuming matches move the payload out of the box (`takeValue`) and
// rebuild: `reverse` on a move-only list, `map` on a Copyable one. Every
// move-only element is dropped exactly once.

module Test

import std.numeric.(Int64)

var drops: Int64 = 0;

struct Token: not Copyable {
    var n: Int64
    deinit { drops = drops + 1; }
}

indirect enum Tokens: not Copyable {
    case End
    case Link(Token, Tokens)
}

func reverse(consuming list: Tokens) -> Tokens {
    var acc = Tokens.End;
    var rest = list;
    var going = true;
    while going {
        match rest {
            .End => { going = false; rest = Tokens.End; },
            .Link(token, tail) => { acc = .Link(token, acc); rest = tail; }
        }
    }
    acc
}

func first(list: Tokens) -> Int64 {
    match list {
        .End => -1,
        .Link(token, _) => token.n
    }
}

indirect enum Ints {
    case Nil
    case Cons(Int64, Ints)

    consuming func doubled() -> Ints {
        match self {
            .Nil => .Nil,
            .Cons(head, tail) => .Cons(head * 2, tail.doubled())
        }
    }

    func sum() -> Int64 {
        match self {
            .Nil => 0,
            .Cons(head, tail) => head + tail.sum()
        }
    }
}

func tokens() -> Int64 {
    let list = Tokens.Link(Token(n: 1), .Link(Token(n: 2), .Link(Token(n: 3), .End)));
    let reversed = reverse(list);
    if first(reversed) != 3 { return 1 }
    if drops != 0 { return 2 }
    0
}

@main
func main() -> Int64 {
    let r = tokens();
    if r != 0 { return r }
    if drops != 3 { return 10 + drops }
    let ints = Ints.Cons(1, .Cons(2, .Cons(3, .Nil)));
    if ints.doubled().sum() != 12 { return 3 }
    0
}
