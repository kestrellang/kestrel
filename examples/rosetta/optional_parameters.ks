// Rosetta Code: Optional parameters
// https://rosettacode.org/wiki/Optional_parameters
module Main

enum Order {
    case Ascending
    case Descending
}

// Sort a table by one column; column, order and ordering function all have defaults.
func sortTable(table: Array[Array[String]], column col: Int64 = 0, ordering order: Order = .Ascending) -> Array[Array[String]] {
    var rows = table.sorted(by: { (a, b) in a(col) < b(col) });
    if let .Descending = order { rows.reverse(); }
    rows
}

@main
func main() {
    let table = [["a", "b", "c"], ["", "q", "z"], ["zap", "zip", "Zot"]];
    println(sortTable(table));
    println(sortTable(table, column: 2));
    println(sortTable(table, column: 1, ordering: .Descending));
}
