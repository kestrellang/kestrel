// Rosetta Code: Digital root
// https://rosettacode.org/wiki/Digital_root
module Main

func digitSum(n: Int64) -> Int64 {
    var rest = n;
    var sum = 0;
    while rest > 0 {
        sum = sum + rest % 10;
        rest = rest / 10;
    }
    sum
}

// Returns (additive persistence, digital root).
func digitalRoot(n: Int64) -> (Int64, Int64) {
    var value = n;
    var persistence = 0;
    while value >= 10 {
        value = digitSum(value);
        persistence = persistence + 1;
    }
    (persistence, value)
}

@main
func main() {
    for n in [627615, 39390, 588225, 393900588225] {
        let (persistence, root) = digitalRoot(n);
        println("\(n) has additive persistence \(persistence) and digital root \(root)");
    }
}
