// Rosetta Code: Hamming numbers
// https://rosettacode.org/wiki/Hamming_numbers
module Main

// Dijkstra's three-pointer merge of 2h, 3h and 5h.
func hamming(count: Int64) -> Array[Int64] {
    var h: Array[Int64] = [1];
    var i2 = 0;
    var i3 = 0;
    var i5 = 0;
    while h.count < count {
        let next2 = h(i2) * 2;
        let next3 = h(i3) * 3;
        let next5 = h(i5) * 5;
        var next = next2;
        if next3 < next { next = next3; }
        if next5 < next { next = next5; }
        h.append(next);
        if next == next2 { i2 = i2 + 1; }
        if next == next3 { i3 = i3 + 1; }
        if next == next5 { i5 = i5 + 1; }
    }
    h
}

@main
func main() {
    let h = hamming(1691);
    println("First 20: \(h.prefix(20).toOwned())");
    println("1691st: \(h(1690))");
}
