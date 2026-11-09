// Rosetta Code: Base64 encode data
// https://rosettacode.org/wiki/Base64_encode_data
module Main

let alphabet: String = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

func encode(bytes: Array[UInt8]) -> String {
    let table = alphabet.chars.iter().collect();
    var out = StringBuilder();
    for chunk in bytes.chunks(of: 3).toArray() {
        // Pack up to three bytes into 24 bits, then emit four 6-bit digits.
        var n = 0;
        for i in 0..<3 {
            n = n << 8;
            if i < chunk.count { n = n | Int64(from: chunk(i)); }
        }
        for i in 0..<4 {
            if i <= chunk.count {
                out.append(char: table((n >> (18 - 6 * i)) & 63));
            } else {
                out.append("=");
            }
        }
    }
    out.build()
}

func decode(text: String) -> Array[UInt8] {
    let table = alphabet.chars.iter().collect();
    var out = Array[UInt8]();
    var n = 0;
    var bits = 0;
    for c in text.chars {
        guard let some v = table.firstIndex(of: c) else { continue }
        n = (n << 6) | v;
        bits = bits + 6;
        if bits >= 8 {
            bits = bits - 8;
            out.append(UInt8(truncating: (n >> bits) & 255));
        }
    }
    out
}

@main
func main() {
    for s in ["Hello, World!", "Man", "Ma", "M", "Rosetta Code Base64 encode data task"] {
        let encoded = encode(s.toBytes());
        println("\(s) -> \(encoded) -> \(String(fromUtf8Lossy: decode(encoded)))");
    }
}
