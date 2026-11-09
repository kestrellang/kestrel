// Rosetta Code: Power set
// https://rosettacode.org/wiki/Power_set
module Main

// Every subset is a bitmask over the elements.
func powerSet[T](items: Array[T]) -> Array[Array[T]] where T: Copyable {
    var result = Array[Array[T]]();
    for mask in 0..<(1 << items.count) {
        var subset = Array[T]();
        for (i, item) in items.iter().enumerate() {
            if (mask >> i) & 1 == 1 { subset.append(item); }
        }
        result.append(subset);
    }
    result
}

@main
func main() {
    for subset in powerSet([1, 2, 3, 4]) {
        println(subset);
    }
    println("power set of [] has \(powerSet(Array[Int64]()).count) element");
}
