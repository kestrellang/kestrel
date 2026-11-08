// Rosetta Code: String matching
// https://rosettacode.org/wiki/String_matching
module Main

@main
func main() {
    let s = "abcd abcd";
    println("starts with \"ab\": \(s.starts(with: "ab"))");
    println("ends with \"cd\": \(s.ends(with: "cd"))");
    println("contains \"bc\": \(s.contains("bc"))");
    println("contains \"xy\": \(s.contains("xy"))");
    match s.firstIndex(of: "cd") {
        .Some(i) => println("first \"cd\" at byte \(i.value)"),
        .None => println("no \"cd\""),
    }
    match s.lastIndex(of: "cd") {
        .Some(i) => println("last \"cd\" at byte \(i.value)"),
        .None => println("no \"cd\""),
    }
}
