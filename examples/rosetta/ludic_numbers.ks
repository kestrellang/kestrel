// Rosetta Code: Ludic numbers
// https://rosettacode.org/wiki/Ludic_numbers
module Main

func ludics(upTo limit: Int64) -> Array[Int64] {
    var result: Array[Int64] = [1];
    var candidates = Array(from: 2..=limit);
    while not candidates.isEmpty {
        let first = candidates(0);
        result.append(first);
        // Remove every first-th remaining number, starting with the first.
        var next = Array[Int64]();
        for (i, x) in candidates.iter().enumerate() {
            if i % first != 0 { next.append(x); }
        }
        candidates = next;
    }
    result
}

@main
func main() {
    let l = ludics(upTo: 22000);
    println("First 25: \(l.prefix(25).toArray())");
    println("Ludic numbers <= 1000: \(l.iter().filter { it <= 1000 }.count())");
    println("2000th..2005th: \(l(1999..<2005).toArray())");
    let triplets = l.iter().filter { (x) in x + 6 < 250 and l.contains(x + 2) and l.contains(x + 6) }.collect();
    println("Triplets below 250: \(triplets.map { "(\(it), \(it + 2), \(it + 6))" }.joined(" "))");
}
