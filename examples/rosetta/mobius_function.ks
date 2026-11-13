// Rosetta Code: Möbius function
// https://rosettacode.org/wiki/M%C3%B6bius_function
module Main

func mobius(n: Int64) -> Int64 {
    var rest = n;
    var sign = 1;
    var p = 2;
    while p * p <= rest {
        if rest % p == 0 {
            rest = rest / p;
            if rest % p == 0 { return 0 }   // square factor
            sign = -sign;
        }
        p = p + 1;
    }
    if rest > 1 { sign = -sign; }
    sign
}

@main
func main() {
    println("First 199 terms:");
    var line = StringBuilder();
    line.append("   ");
    for n in 1..<200 {
        line.append("\(mobius(n):>3)");
        if n % 20 == 19 {
            println(line.build());
            line.clear();
        }
    }
    if not line.isEmpty { println(line.build()); }
}
