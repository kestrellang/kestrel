// Rosetta Code: Null object
// https://rosettacode.org/wiki/Null_object
// There is no null; absence is spelled Optional.
module Main

func find(name: String, in names: Array[String]) -> Int64? {
    names.firstIndex(of: name)
}

@main
func main() {
    let names = ["ann", "bob", "cy"];
    let missing: Int64? = null;
    println("missing is None: \(missing == null)");
    for name in ["bob", "dee"] {
        match find(name, in: names) {
            some i => println("\(name) is at \(i)"),
            null => println("\(name) is not there"),
        }
    }
    // Defaulting and chaining.
    println("dee at \(find("dee", in: names) ?? -1)");
    println("cy doubled: \((find("cy", in: names)?.multiply(2)) ?? -1)");
}
