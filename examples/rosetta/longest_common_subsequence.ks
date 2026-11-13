// Rosetta Code: Longest common subsequence
// https://rosettacode.org/wiki/Longest_common_subsequence
module Main

func lcs(a: String, b: String) -> String {
    let x = a.chars.iter().collect();
    let y = b.chars.iter().collect();
    // lengths(i)(j) = LCS length of x(i..) and y(j..).
    var lengths = Array(of: x.count + 1, generatedBy: { (_) in Array[Int64](repeating: 0, count: y.count + 1) });
    var i = x.count - 1;
    while i >= 0 {
        var j = y.count - 1;
        while j >= 0 {
            lengths(i)(j) = if x(i) == y(j) {
                lengths(i + 1)(j + 1) + 1
            } else if lengths(i + 1)(j) >= lengths(i)(j + 1) {
                lengths(i + 1)(j)
            } else {
                lengths(i)(j + 1)
            };
            j = j - 1;
        }
        i = i - 1;
    }
    // Walk the table forwards to recover one subsequence.
    var out = StringBuilder();
    var (p, q) = (0, 0);
    while p < x.count and q < y.count {
        if x(p) == y(q) {
            out.append(char: x(p));
            p = p + 1;
            q = q + 1;
        } else if lengths(p + 1)(q) >= lengths(p)(q + 1) {
            p = p + 1;
        } else {
            q = q + 1;
        }
    }
    out.build()
}

@main
func main() {
    println(lcs("1234", "1224533324"));
    println(lcs("thisisatest", "testing123testing"));
}
