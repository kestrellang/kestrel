// Rosetta Code: Named parameters
// https://rosettacode.org/wiki/Named_parameters
module Main

// External labels are part of the call; defaults make them optional.
func greet(name: String, greeting word: String = "Hello", punctuation mark: String = "!") -> String {
    "\(word), \(name)\(mark)"
}

@main
func main() {
    println(greet("Ada"));
    println(greet("Ada", greeting: "Welcome"));
    println(greet("Ada", punctuation: "?"));
    println(greet("Ada", greeting: "Bye", punctuation: "."));
}
