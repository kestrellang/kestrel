// Rosetta Code: Singly-linked list/Traversal
// https://rosettacode.org/wiki/Singly-linked_list/Traversal
module Main

indirect enum List[T] {
    case Cons(head: T, tail: List[T])
    case Nil
}

func fromArray(items: Array[Int64]) -> List[Int64] {
    var list = List[Int64].Nil;
    var i = items.count - 1;
    while i >= 0 {
        list = .Cons(head: items(i), tail: list);
        i = i - 1;
    }
    list
}

func printAll(list: List[Int64]) {
    match list {
        .Cons(head, tail) => {
            println(head);
            printAll(tail);
        },
        .Nil => {}
    }
}

func sum(list: List[Int64]) -> Int64 {
    match list {
        .Cons(head, tail) => head + sum(tail),
        .Nil => 0,
    }
}

@main
func main() {
    let list = fromArray([1, 2, 3, 4, 5]);
    printAll(list);
    println("sum = \(sum(list))");
}
