// Rosetta Code: Rot-13
// https://rosettacode.org/wiki/Rot-13
module Main

func rot13(c: Char) -> Char {
    match c {
        'a'..='m' or 'A'..='M' => Char(unchecked: c.value() + 13),
        'n'..='z' or 'N'..='Z' => Char(unchecked: c.value() - 13),
        _ => c,
    }
}

@main
func main() {
    let text = "Why did the chicken cross the road? Gb trg gb gur bgure fvqr!";
    println(String(from: text.chars.iter().map { rot13(it) }.collect()));
}
