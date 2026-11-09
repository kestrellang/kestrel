// Rosetta Code: Pi
// https://rosettacode.org/wiki/Pi
// Rabinowitz-Wagon spigot: prints the first digits of pi using only integers.
module Main

@main
func main() {
    let digits = 200;
    let len = digits * 10 / 3 + 1;
    var a = Array[Int64](repeating: 2, count: len);
    var out = StringBuilder();
    var nines: Int64 = 0;
    var predigit: Int64 = 0;
    for j in 0..<digits {
        var q: Int64 = 0;
        var i = len;
        while i > 0 {
            let x = 10 * a(i - 1) + q * i;
            a(i - 1) = x % (2 * i - 1);
            q = x / (2 * i - 1);
            i = i - 1;
        }
        a(0) = q % 10;
        q = q / 10;
        if q == 9 {
            nines = nines + 1;
        } else if q == 10 {
            out.append("\(predigit + 1)");
            for _ in 0..<nines { out.append("0"); }
            predigit = 0;
            nines = 0;
        } else {
            if j > 0 { out.append("\(predigit)"); }
            predigit = q;
            for _ in 0..<nines { out.append("9"); }
            nines = 0;
        }
    }
    out.append("\(predigit)");
    let s = out.build();
    println("3.\(s.chars.substring(1..))");
}
