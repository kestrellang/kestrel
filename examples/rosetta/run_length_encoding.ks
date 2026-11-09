// Rosetta Code: Run-length encoding
// https://rosettacode.org/wiki/Run-length_encoding
module Main

func encode(s: String) -> String {
    let chars = s.chars.iter().collect();
    var out = StringBuilder();
    var i = 0;
    while i < chars.count {
        var run = 1;
        while i + run < chars.count and chars(i + run) == chars(i) { run = run + 1; }
        out.append("\(run)");
        out.append(char: chars(i));
        i = i + run;
    }
    out.build()
}

func decode(s: String) -> String {
    var out = StringBuilder();
    var count = 0;
    for c in s.chars {
        match c.digitValue() {
            some d => count = count * 10 + Int64(from: d),
            null => {
                out.append(c.toString().repeated(count));
                count = 0;
            }
        }
    }
    out.build()
}

@main
func main() {
    let input = "WWWWWWWWWWWWBWWWWWWWWWWWWBBBWWWWWWWWWWWWWWWWWWWWWWWWBWWWWWWWWWWWWWW";
    let encoded = encode(input);
    println(encoded);
    println(decode(encoded));
    println("round trip ok: \(decode(encoded) == input)");
}
