// Rosetta Code: Two sum
// https://rosettacode.org/wiki/Two_sum
module Main

// The input is sorted, so walk inwards from both ends.
func twoSum(a: Array[Int64], target: Int64) -> (Int64, Int64)? {
    var i: Int64 = 0;
    var j = a.count - 1;
    while i < j {
        let s = a(i) + a(j);
        if s == target { return (i, j) }
        if s < target { i = i + 1; } else { j = j - 1; }
    }
    null
}

@main
func main() {
    let a = [0, 2, 11, 19, 90];
    match twoSum(a, 21) {
        some (i, j) => println("[\(i), \(j)]"),
        null => println("[]"),
    }
    match twoSum(a, 25) {
        some (i, j) => println("[\(i), \(j)]"),
        null => println("no pair sums to 25"),
    }
}
