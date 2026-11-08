// Rosetta Code: Chinese remainder theorem
// https://rosettacode.org/wiki/Chinese_remainder_theorem
module Main

func modInverse(a: Int64, m: Int64) -> Int64? {
    var oldR = a.modulo(by: m);
    var r = m;
    var oldS: Int64 = 1;
    var s: Int64 = 0;
    while r != 0 {
        let q = oldR / r;
        let nextR = oldR - q * r;
        oldR = r;
        r = nextR;
        let nextS = oldS - q * s;
        oldS = s;
        s = nextS;
    }
    if oldR != 1 { return null }
    oldS.modulo(by: m)
}

func crt(residues: Array[Int64], moduli: Array[Int64]) -> Int64? {
    let product = moduli.iter().product();
    var sum: Int64 = 0;
    for (r, m) in residues.iter().zip(moduli.iter()) {
        let p = product / m;
        let inv = try modInverse(p, m);
        sum = sum + r * inv * p;
    }
    sum.modulo(by: product)
}

@main
func main() {
    println(crt([2, 3, 2], [3, 5, 7]) ?? -1);
    match crt([1, 2], [4, 6]) {
        some x => println(x),
        null => println("no solution: moduli aren't coprime"),
    }
}
