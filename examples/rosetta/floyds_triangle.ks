// Rosetta Code: Floyd's triangle
// https://rosettacode.org/wiki/Floyd%27s_triangle
module Main

func floyd(rows: Int64) {
    // Each column is as wide as the number in it on the last row.
    let lastStart = rows * (rows - 1) / 2 + 1;
    let widths = (0..<rows).iter().map { "\(lastStart + it)".chars.count }.collect();
    var n = 1;
    for row in 1..=rows {
        var cells = Array[String]();
        for col in 0..<row {
            cells.append("\(n)".padded(toWidth: widths(col), side: .Leading));
            n = n + 1;
        }
        println(cells.joined(by: " "));
    }
}

@main
func main() {
    floyd(5);
    println();
    floyd(14);
}
