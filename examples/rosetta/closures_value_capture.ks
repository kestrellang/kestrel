// Rosetta Code: Closures/Value capture
// https://rosettacode.org/wiki/Closures/Value_capture
module Main

@main
func main() {
    var funcs = Array[escaping () -> Int64]();
    for i in 0..<10 {
        // Each escaping closure snapshots the current `i`.
        funcs.append({ () in i * i });
    }
    for i in 0..<9 {
        let f = funcs(i);
        println("funcs(\(i))() = \(f())");
    }
}
