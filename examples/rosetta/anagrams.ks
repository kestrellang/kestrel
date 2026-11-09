// Rosetta Code: Anagrams
// https://rosettacode.org/wiki/Anagrams
// (The task uses unixdict.txt; a small embedded word list keeps this self-contained.)
module Main

@main
func main() {
    let words = ["abel", "able", "bale", "bela", "elba", "alger", "glare", "lager", "large",
                 "regal", "angel", "angle", "galen", "glean", "lange", "caret", "carte",
                 "cater", "crate", "trace", "elan", "lane", "lean", "lena", "neal", "evil",
                 "levi", "live", "veil", "vile", "kestrel", "falcon", "hawk"];
    // Group words by their sorted letters.
    let groups = Dictionary[String, Array[String]](grouping: words, by: { (w: String) in
        w.chars.iter().collect().sorted().joined("")
    });
    let largest = groups.values.iter().map { it.count }.max() ?? 0;
    let best = groups.values.iter()
        .filter { it.count == largest }
        .map { it.sorted() }
        .collect()
        .sorted(by: { (a, b) in a(0) < b(0) });
    for group in best {
        println(group.joined(" "));
    }
}
