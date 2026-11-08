// Rosetta Code: String concatenation
// https://rosettacode.org/wiki/String_concatenation
module Main

@main
func main() {
    let s = "hello";
    println(s + " literal");
    let s2 = s + " world";
    println(s2);
    var s3 = s2;
    s3.append("!");
    println(s3);
    println("\(s), \(s2)");
}
