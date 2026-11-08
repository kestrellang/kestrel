// Rosetta Code: Boolean values
// https://rosettacode.org/wiki/Boolean_values
// Bool is its own type: only `true` and `false`, and conditions must be Bool.
module Main

@main
func main() {
    let t = true;
    let f = false;
    println("t = \(t), f = \(f)");
    println("not t = \(not t)");
    println("t and f = \(t and f), t or f = \(t or f)");
    println("t == f: \(t == f)");
}
