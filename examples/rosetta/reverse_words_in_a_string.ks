// Rosetta Code: Reverse words in a string
// https://rosettacode.org/wiki/Reverse_words_in_a_string
module Main

@main
func main() {
    let text = """
        ---------- Ice and Fire ------------

        fire, in end will world the say Some
        ice. in say Some

        desire of tasted I've what From
        fire. favor who those with hold I

        ... elided paragraph last ...

        Frost Robert -----------------------
        """;
    for line in text.lines {
        var words = line.split(where: { it.isWhitespace }).iter()
            .filter { not it.isEmpty }
            .map { it.toOwned() }
            .collect();
        words.reverse();
        println(words.joined(" "));
    }
}
