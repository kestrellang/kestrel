// Rosetta Code: Flatten a list
// https://rosettacode.org/wiki/Flatten_a_list
module Main

// An arbitrarily nested list of integers.
indirect enum Nested {
    case Item(Int64)
    case List(Array[Nested])
}

func flatten(n: Nested, mutating out: Array[Int64]) {
    match n {
        .Item(x) => out.append(x),
        .List(items) => {
            for item in items { flatten(item, out); }
        }
    }
}

func item(x: Int64) -> Nested { .Item(x) }
func list(items: Array[Nested]) -> Nested { .List(items) }

@main
func main() {
    // [[1], 2, [[3, 4], 5], [[[]]], [[[6]]], 7, 8, []]
    let nested = list([
        list([item(1)]), item(2), list([list([item(3), item(4)]), item(5)]),
        list([list([list([])])]), list([list([list([item(6)])])]), item(7), item(8), list([]),
    ]);
    var out = Array[Int64]();
    flatten(nested, out);
    println(out);
}
