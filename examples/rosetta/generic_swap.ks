// Rosetta Code: Generic swap
// https://rosettacode.org/wiki/Generic_swap
module Main

func swap[T](mutating a: T, mutating b: T) where T: Copyable {
    let tmp = a;
    a = b;
    b = tmp;
}

@main
func main() {
    var x = 1;
    var y = 2;
    swap(x, y);
    println("x = \(x), y = \(y)");
    var s = "hello";
    var t = "world";
    swap(s, t);
    println("s = \(s), t = \(t)");
}
