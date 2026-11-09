// Rosetta Code: Look-and-say sequence
// https://rosettacode.org/wiki/Look-and-say_sequence
module Main

func lookAndSay(s: String) -> String {
    let chars = s.chars.iter().collect();
    var out = StringBuilder();
    var i = 0;
    while i < chars.count {
        var run = 1;
        while i + run < chars.count and chars(i + run) == chars(i) {
            run = run + 1;
        }
        out.append("\(run)");
        out.append(char: chars(i));
        i = i + run;
    }
    out.build()
}

@main
func main() {
    var s = "1";
    for _ in 0..<12 {
        println(s);
        s = lookAndSay(s);
    }
}
