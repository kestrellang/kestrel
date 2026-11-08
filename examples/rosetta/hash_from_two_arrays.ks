// Rosetta Code: Hash from two arrays
// https://rosettacode.org/wiki/Hash_from_two_arrays
module Main

@main
func main() {
    let keys = ["a", "b", "c"];
    let values = [1, 2, 3];
    let hash = Dictionary[String, Int64](from: keys.iter().zip(values.iter()).collect());
    for key in keys {
        println("\(key): \(hash(key) ?? 0)");
    }
}
