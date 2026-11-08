// Rosetta Code: Kolakoski sequence
// https://rosettacode.org/wiki/Kolakoski_sequence
module Main

func kolakoski(digits: Array[Int64], length: Int64) -> Array[Int64] {
    var seq = Array[Int64]();
    var i: Int64 = 0;
    var d: Int64 = 0;
    while seq.count < length {
        let next = digits(d % digits.count);
        seq.append(next);
        // The i-th term says how long the current run is.
        for _ in 1..<seq(i) {
            if seq.count >= length { break }
            seq.append(next);
        }
        i = i + 1;
        d = d + 1;
    }
    seq
}

func isKolakoski(seq: Array[Int64]) -> Bool {
    // Run-length encode and check it reproduces a prefix of the sequence.
    var runs = Array[Int64]();
    var i: Int64 = 0;
    while i < seq.count {
        var j = i;
        while j < seq.count and seq(j) == seq(i) { j = j + 1; }
        runs.append(j - i);
        i = j;
    }
    runs.removeSubrange((runs.count - 1)..<runs.count);
    runs.iter().enumerate().all { (p) in p.1 == seq(p.0) }
}

@main
func main() {
    for (digits, len) in [([1, 2], 20), ([2, 1], 20), ([1, 3, 1, 2], 30), ([1, 3, 2, 1], 30)] {
        let seq = kolakoski(digits, len);
        println("\(digits): \(seq.joined(""))  kolakoski: \(isKolakoski(seq))");
    }
}
