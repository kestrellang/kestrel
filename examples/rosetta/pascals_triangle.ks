// Rosetta Code: Pascal's triangle
// https://rosettacode.org/wiki/Pascal%27s_triangle
module Main

@main
func main() {
    let rows = 10;
    var row = [1];
    for n in 0..<rows {
        let indent = " ".repeated(2 * (rows - n - 1));
        let cells = row.map { "\(it:>3)" };
        println("\(indent)\(cells.joined(by: " "))");
        // Each new entry is the sum of the two above it.
        var next = [1];
        for i in 1..<row.count { next.append(row(i - 1) + row(i)); }
        next.append(1);
        row = next;
    }
}
