// Rosetta Code: Sierpinski triangle
// https://rosettacode.org/wiki/Sierpinski_triangle
module Main

@main
func main() {
    let order = 4;
    let size = 1 << order;
    // Cell (x, y) is filled when x AND y == 0 (Pascal's triangle mod 2).
    for y in 0..<size {
        var line = StringBuilder();
        line.append(" ".repeated(size - y - 1));
        for x in 0..=y {
            line.append(if x & (y - x) == 0 { "* " } else { "  " });
        }
        println(line.build().trimmedEnd().toOwned());
    }
}
