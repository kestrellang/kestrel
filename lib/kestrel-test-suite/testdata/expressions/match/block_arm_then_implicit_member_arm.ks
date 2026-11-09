// test: execution
// stdlib: true
// expect-stdout: 1\n2\nend\n

// A block arm `=> { … }` with no trailing comma, followed by an arm whose
// pattern starts with `.`, should end the block arm. Today the parser reads
// `{ … }\n.Nil` as member access on the block (E102 "no member 'Nil' on type
// '() -> ()'") and then fails on `=>`.
// Found while writing examples/rosetta (singly-linked list traversal).

module Test

indirect enum List {
    case Cons(Int64, List)
    case Nil
}

func walk(l: List) {
    match l {
        .Cons(head, tail) => {
            println(head);
            walk(tail);
        }
        .Nil => { println("end"); }
    }
}

@main
func main() {
    walk(.Cons(1, .Cons(2, .Nil)));
}
