// Rosetta Code: Jewels and stones
// https://rosettacode.org/wiki/Jewels_and_stones
module Main

func countJewels(stones: String, jewels: String) -> Int64 {
    stones.chars.iter().filter { jewels.chars.iter().contains(it) }.count()
}

@main
func main() {
    println(countJewels("aAAbbbb", "aA"));
    println(countJewels("ZZ", "z"));
}
