// Rosetta Code: Langton's ant
// https://rosettacode.org/wiki/Langton%27s_ant
module Main

@main
func main() {
    let size = 100;
    var black = Array[Bool](repeating: false, count: size * size);
    var (x, y) = (size / 2, size / 2);
    // Directions: 0 up, 1 right, 2 down, 3 left.
    var dir = 0;
    var steps = 0;
    while x >= 0 and x < size and y >= 0 and y < size {
        let i = y * size + x;
        // White: turn right. Black: turn left. Then flip the cell.
        dir = if black(i) { (dir + 3) % 4 } else { (dir + 1) % 4 };
        black(i) = not black(i);
        match dir {
            0 => y = y - 1,
            1 => x = x + 1,
            2 => y = y + 1,
            _ => x = x - 1,
        }
        steps = steps + 1;
    }
    println("The ant left the grid after \(steps) steps.");
    // Print the interesting part of the grid.
    for row in 28..<80 {
        let line = (17..<83).iter().map { if black(row * size + it) { "#" } else { " " } }.collect();
        println(line.joined("").trimmedEnd().toOwned());
    }
}
