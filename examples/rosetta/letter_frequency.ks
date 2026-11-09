// Rosetta Code: Letter frequency
// https://rosettacode.org/wiki/Letter_frequency
module Main

@main
func main() {
    let text = """
        It was the best of times, it was the worst of times, it was the age of
        wisdom, it was the age of foolishness, it was the epoch of belief, it
        was the epoch of incredulity, it was the season of Light, it was the
        season of Darkness.
        """;
    var counts = Array[Int64](repeating: 0, count: 26);
    for c in text.chars {
        if c.isAsciiLetter {
            let i = Int64(from: c.lowercased().value() - 'a'.value());
            counts(i) = counts(i) + 1;
        }
    }
    for i in 0..<26 {
        if counts(i) > 0 {
            let letter = Char(unchecked: 'a'.value() + UInt32(truncating: i));
            println("\(letter): \(counts(i))");
        }
    }
}
