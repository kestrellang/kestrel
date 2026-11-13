// Rosetta Code: 99 bottles of beer
// https://rosettacode.org/wiki/99_bottles_of_beer
module Main

func bottles(n: Int64) -> String {
    let s = match n {
        0 => "No more bottles",
        1 => "1 bottle",
        _ => "\(n) bottles",
    };
    s
}

@main
func main() {
    var n = 99;
    while n > 0 {
        println("\(bottles(n)) of beer on the wall");
        println("\(bottles(n)) of beer");
        println("Take one down, pass it around");
        println("\(bottles(n - 1).lowercased()) of beer on the wall");
        println();
        n = n - 1;
    }
}
