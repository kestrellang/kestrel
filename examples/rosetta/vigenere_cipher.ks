// Rosetta Code: Vigenère cipher
// https://rosettacode.org/wiki/Vigen%C3%A8re_cipher
module Main

// Letters only, uppercased; everything else is dropped.
func letters(s: String) -> Array[Int64] {
    s.uppercased().chars.iter()
        .filter { it.isAsciiUppercase }
        .map { Int64(from: it.value() - 'A'.value()) }
        .collect()
}

func toText(codes: Array[Int64]) -> String {
    codes.map { Char(unchecked: 'A'.value() + UInt32(truncating: it)) }.joined(by: "")
}

func vigenere(text: String, key: String, decrypt: Bool) -> String {
    let k = letters(key);
    let sign: Int64 = if decrypt { -1 } else { 1 };
    let codes = letters(text);
    toText(Array(of: codes.count, generatedBy: { (i: Int64) in (codes(i) + sign * k(i % k.count)).modulo(by: 26) }))
}

@main
func main() {
    let text = "Beware the Jabberwock, my son! The jaws that bite, the claws that catch!";
    let key = "VIGENERECIPHER";
    let encrypted = vigenere(text, key, false);
    println("Original:  \(text)");
    println("Encrypted: \(encrypted)");
    println("Decrypted: \(vigenere(encrypted, key, true))");
}
