// Rosetta Code: Sierpinski carpet
// https://rosettacode.org/wiki/Sierpinski_carpet
module Main

// A cell is empty if, at any scale, both coordinates have a middle-third digit.
func inCarpet(x: Int64, y: Int64) -> Bool {
    var (a, b) = (x, y);
    while a > 0 or b > 0 {
        if a % 3 == 1 and b % 3 == 1 { return false }
        a = a / 3;
        b = b / 3;
    }
    true
}

@main
func main() {
    let size = 27;
    for y in 0..<size {
        println((0..<size).iter().map { if inCarpet(it, y) { "#" } else { " " } }.collect().joined(""));
    }
}
