// Rosetta Code: Multiplication tables
// https://rosettacode.org/wiki/Multiplication_tables
module Main

@main
func main() {
    let n = 12;
    var header = StringBuilder();
    header.append("  x |");
    for j in 1..=n { header.append("\(j:>4)"); }
    println(header.build());
    println("----+\("-".repeated(4 * n))");
    for i in 1..=n {
        var row = StringBuilder();
        row.append("\(i:>3) |");
        for j in 1..=n {
            // Only the upper triangle, as the task asks.
            if j < i { row.append("    "); } else { row.append("\(i * j:>4)"); }
        }
        println(row.build());
    }
}
