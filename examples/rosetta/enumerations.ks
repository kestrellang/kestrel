// Rosetta Code: Enumerations
// https://rosettacode.org/wiki/Enumerations
module Main

enum Fruit {
    case Apple
    case Banana
    case Cherry

    // Enums can carry methods; explicit values come from a match.
    func value() -> Int64 {
        match self {
            .Apple => 1,
            .Banana => 2,
            .Cherry => 4,
        }
    }

    func name() -> String {
        match self {
            .Apple => "apple",
            .Banana => "banana",
            .Cherry => "cherry",
        }
    }
}

@main
func main() {
    let all: Array[Fruit] = [.Apple, .Banana, .Cherry];
    for fruit in all {
        println("\(fruit.name()) = \(fruit.value())");
    }
}
