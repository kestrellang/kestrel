// Rosetta Code: Caesar cipher
// https://rosettacode.org/wiki/Caesar_cipher
module Main

func shift(c: Char, by key: Int64) -> Char {
    let base: UInt32 = if c.isAsciiUppercase { 65 } else if c.isAsciiLowercase { 97 } else { return c };
    let offset = (Int64(from: c.value() - base) + key).modulo(by: 26);
    Char(unchecked: base + UInt32(truncating: offset))
}

func caesar(text: String, key: Int64) -> String {
    text.chars.iter().map { shift(it, by: key) }.collect().joined("")
}

@main
func main() {
    let plain = "The quick brown fox Jumped over the lazy Dog";
    let encoded = caesar(plain, 11);
    println(encoded);
    println(caesar(encoded, -11));
}
