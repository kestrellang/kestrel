// Rosetta Code: Array concatenation
// https://rosettacode.org/wiki/Array_concatenation
module Main

@main
func main() {
    let a = [1, 2, 3];
    let b = [4, 5, 6];
    var c = a;
    c.append(contentsOf: b);
    println(c);
    // Or flatten a list of arrays.
    println([a, b, [7]].flatten());
}
