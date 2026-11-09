// Rosetta Code: LZW compression
// https://rosettacode.org/wiki/LZW_compression
module Main

func compress(text: String) -> Array[Int64] {
    var dict = Dictionary[String, Int64]();
    for i in 0..<256 { dict.insert(Char(unchecked: UInt32(truncating: i)).toString(), i); }
    var w = "";
    var out = Array[Int64]();
    for c in text.chars {
        let wc = w + c.toString();
        if dict.contains(wc) {
            w = wc;
        } else {
            out.append(dict(w) ?? 0);
            dict.insert(wc, dict.count);
            w = c.toString();
        }
    }
    if not w.isEmpty { out.append(dict(w) ?? 0); }
    out
}

func decompress(codes: Array[Int64]) -> String {
    var dict = Array(of: 256, generatedBy: { Char(unchecked: UInt32(truncating: it)).toString() });
    var w = dict(codes(0));
    var out = StringBuilder();
    out.append(w);
    for k in codes.drop(first: 1) {
        let entry = if k < dict.count { dict(k) } else { w + w.chars.substring(..<1) };
        out.append(entry);
        dict.append(w + entry.chars.substring(..<1));
        w = entry;
    }
    out.build()
}

@main
func main() {
    let text = "TOBEORNOTTOBEORTOBEORNOT";
    let codes = compress(text);
    println(codes);
    println(decompress(codes));
}
