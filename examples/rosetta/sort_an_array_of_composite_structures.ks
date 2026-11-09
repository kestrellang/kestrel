// Rosetta Code: Sort an array of composite structures
// https://rosettacode.org/wiki/Sort_an_array_of_composite_structures
module Main

struct Element {
    let name: String;
    let value: String;
}

@main
func main() {
    var elements = [
        Element(name: "Tom", value: "Cat"), Element(name: "Joe", value: "Dog"),
        Element(name: "Alice", value: "Rabbit"), Element(name: "Bob", value: "Hamster"),
    ];
    elements.sort(byKey: { it.name });
    for e in elements {
        println("\(e.name:<6) \(e.value)");
    }
}
