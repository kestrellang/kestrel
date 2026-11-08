// Rosetta Code: Abundant, deficient and perfect number classifications
// https://rosettacode.org/wiki/Abundant,_deficient_and_perfect_number_classifications
module Main

@main
func main() {
    let limit = 20000;
    var sums = Array[Int64](repeating: 0, count: limit + 1);
    for d in 1..=(limit / 2) {
        var m = 2 * d;
        while m <= limit {
            sums(m) = sums(m) + d;
            m = m + d;
        }
    }
    var (deficient, perfect, abundant) = (0, 0, 0);
    for n in 1..=limit {
        let s = sums(n);
        if s < n { deficient = deficient + 1; }
        else if s == n { perfect = perfect + 1; }
        else { abundant = abundant + 1; }
    }
    println("Deficient: \(deficient)");
    println("Perfect:   \(perfect)");
    println("Abundant:  \(abundant)");
}
